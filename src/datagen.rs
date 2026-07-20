use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use arrow_array::{ArrayRef, BinaryArray, Int8Array, Int16Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;
use pyrrhic_rs::{EngineAdapter, TableBases, WdlProbeResult};
use rand::Rng;
use regex::{Captures, Regex};

use super::attacks;
use super::bitboard::*;
use super::board::*;
use super::move_list::*;
use super::piece::*;
use super::search::*;
use super::square::*;
use super::timer::*;
use super::traits::*;
use super::tt::*;
use super::types::*;

pub fn run(args: Vec<String>) {
    let cfg = match Config::parse(&args) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            std::process::exit(2);
        }
    };

    match DataGen::new(cfg) {
        Ok(datagen) => datagen.run(),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    }
}

struct DataGen {
    cfg: Config,
    syzygy: Option<Syzygy>,
    produced: AtomicU64,
}

impl DataGen {
    fn new(cfg: Config) -> Result<Self, String> {
        let syzygy = cfg
            .syzygy
            .as_deref()
            .map(|path| Syzygy::load(path, cfg.tb_men))
            .transpose()?;

        if let Some(syzygy) = &syzygy {
            println!(
                "Loaded syzygy tablebases; probing positions with at most {} men.",
                syzygy.men
            );
        }

        Ok(Self {
            cfg,
            syzygy,
            produced: AtomicU64::new(0),
        })
    }

    fn run(&self) {
        std::fs::create_dir_all(&self.cfg.out).expect("Unable to create the output directory.");

        println!(
            "Generating on {} threads at {} soft nodes per move.",
            self.cfg.threads, self.cfg.nodes
        );

        let (tx, rx) = mpsc::channel();
        thread::scope(|s| {
            for _ in 0..self.cfg.threads {
                let tx = tx.clone();
                s.spawn(move || self.worker(tx));
            }
            drop(tx);
            Writer::new(&self.cfg).run(rx);
        });
    }

    fn worker(&self, tx: mpsc::Sender<Game>) {
        let tt = TT::new(self.cfg.hash_mb);
        let mut rng = rand::rng();

        while self.produced.load(Ordering::Relaxed) < self.cfg.positions {
            tt.clear();
            let Some(game) = self
                .play_game(&tt, &mut rng)
                .filter(|game| !game.samples.is_empty())
            else {
                continue;
            };
            self.produced
                .fetch_add(game.samples.len() as u64, Ordering::Relaxed);
            if tx.send(game).is_err() {
                break;
            }
        }
    }

    fn play_game(&self, tt: &TT, rng: &mut impl Rng) -> Option<Game> {
        let mut board = Board::new();
        // Half the games get one extra random ply so that each color is
        // equally often first to move out of the opening; a fixed even count
        // skews outcomes heavily toward white.
        for _ in 0..self.cfg.opening_plies + rng.random_range(0..=1) {
            let moves = MoveList::from::<false>(&board);
            if moves.len() == 0 {
                return None;
            }
            board.push(moves[rng.random_range(0..moves.len())]);
        }

        let mut samples = Vec::new();
        let outcome = loop {
            let moves = MoveList::from::<false>(&board);
            if moves.len() == 0 {
                break if board.in_check() {
                    // The side to move is mated; the previous mover won.
                    if board.ctm() == Color::White { -1 } else { 1 }
                } else {
                    0
                };
            }

            if board.is_draw() || board.ply() >= Self::MAX_GAME_PLIES {
                break 0;
            }

            if let Some(wdl) = self.syzygy.as_ref().and_then(|syzygy| syzygy.probe(&board)) {
                break wdl;
            }

            let timer = Timer::new(
                &board,
                TimeControl::Infinite,
                Arc::new(AtomicBool::new(false)),
                Arc::new(AtomicBool::new(false)),
                Arc::new(AtomicU64::new(0)),
                Duration::ZERO,
            );
            let mut search = Search::new(timer, tt, 1, false);
            let (m, value) = search.go_datagen(&mut board, self.cfg.nodes);
            let m = m.expect("Search returned no move despite legal moves.");

            let white_value = if board.ctm() == Color::White {
                value
            } else {
                -value
            };
            if white_value.is_checkmate() {
                break white_value.signum() as i8;
            }

            let sample = (!board.in_check() && m.is_quiet())
                .then(|| (board.to_string(), white_value.clamp(-16000, 16000) as i16));

            board.push(m);

            if let Some(sample) = sample
                && !board.in_check()
            {
                samples.push(sample);
            }
        };

        Some(Game {
            samples,
            outcome,
            id: rng.random(),
        })
    }
}

impl DataGen {
    const MAX_GAME_PLIES: usize = 800;
}

struct Game {
    samples: Vec<(String, i16)>,
    outcome: i8,
    id: [u8; 16],
}

struct Writer {
    out: PathBuf,
    rows_per_file: usize,
    schema: Arc<Schema>,
    props: WriterProperties,
    fens: Vec<String>,
    game_ids: Vec<[u8; 16]>,
    cps: Vec<i16>,
    outcomes: Vec<i8>,
    games: u64,
    written: u64,
    start: Instant,
}

impl Writer {
    fn new(cfg: &Config) -> Self {
        let schema = Arc::new(Schema::new(vec![
            Field::new("fen", DataType::Utf8, false),
            Field::new("game_id", DataType::Binary, false),
            Field::new("cp", DataType::Int16, false),
            Field::new("outcome", DataType::Int8, false),
        ]));
        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(
                ZstdLevel::try_new(Self::ZSTD_LEVEL).expect("Zstd level should be valid."),
            ))
            .build();

        Self {
            out: cfg.out.clone(),
            rows_per_file: cfg.rows_per_file,
            schema,
            props,
            fens: Vec::new(),
            game_ids: Vec::new(),
            cps: Vec::new(),
            outcomes: Vec::new(),
            games: 0,
            written: 0,
            start: Instant::now(),
        }
    }

    fn run(mut self, rx: mpsc::Receiver<Game>) {
        for game in rx {
            self.games += 1;
            self.push_game(game);

            while self.fens.len() >= self.rows_per_file {
                self.flush(self.rows_per_file);
                let rate = self.written as f64 / self.start.elapsed().as_secs_f64();
                println!(
                    "{} positions from {} games ({rate:.0}/s)",
                    self.written, self.games
                );
            }
        }

        let rest = self.fens.len();
        if rest > 0 {
            self.flush(rest);
        }
        println!(
            "Done: {} positions from {} games.",
            self.written, self.games
        );
    }

    fn push_game(&mut self, game: Game) {
        for (fen, cp) in game.samples {
            self.fens.push(fen);
            self.game_ids.push(game.id);
            self.cps.push(cp);
            self.outcomes.push(game.outcome);
        }
    }

    fn flush(&mut self, n: usize) {
        let columns: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from_iter_values(self.fens.drain(..n))),
            Arc::new(BinaryArray::from_iter_values(self.game_ids.drain(..n))),
            Arc::new(Int16Array::from_iter_values(self.cps.drain(..n))),
            Arc::new(Int8Array::from_iter_values(self.outcomes.drain(..n))),
        ];
        let batch = RecordBatch::try_new(self.schema.clone(), columns)
            .expect("Batch columns should match the schema.");

        let path = self
            .out
            .join(format!("{:032x}.parquet", rand::rng().random::<u128>()));
        let file = File::create(&path).expect("Unable to create the output file.");
        let mut writer = ArrowWriter::try_new(file, self.schema.clone(), Some(self.props.clone()))
            .expect("Unable to create the parquet writer.");
        writer.write(&batch).expect("Unable to write the batch.");
        writer.close().expect("Unable to finish the parquet file.");

        self.written += n as u64;
    }
}

impl Writer {
    const ZSTD_LEVEL: i32 = 22;
}

struct Syzygy {
    tablebases: TableBases<Adapter>,
    men: u32,
}

impl Syzygy {
    // Pyrrhic treats ':' as a path separator, which collides with Windows
    // drive letters; fall back to a path relative to the working directory.
    fn load(path: &str, men: u32) -> Result<Self, String> {
        let attempt = |p: &str| {
            let tablebases = TableBases::<Adapter>::new(p).ok()?;
            // Pyrrhic accepts directories with no tables in them; a KQvK
            // probe (white Ka1/Qb1, black Kh8) proves a real load.
            tablebases
                .probe_wdl(0b11, 1 << 63, (1 << 63) | 1, 0b10, 0, 0, 0, 0, 0, true)
                .is_ok()
                .then_some(tablebases)
        };

        let tablebases = attempt(path)
            .or_else(|| {
                let cwd = std::env::current_dir().ok()?;
                let rel = Self::relative_to(Path::new(path), &cwd)?;
                attempt(rel.to_str()?)
            })
            .ok_or_else(|| format!("No tablebases found at {path}."))?;

        Ok(Self {
            men: men.min(tablebases.max_pieces()),
            tablebases,
        })
    }

    fn probe(&self, board: &Board) -> Option<i8> {
        if u32::from(board.all_pieces().pop_count()) > self.men || board.has_castling_rights() {
            return None;
        }

        let wdl = self
            .tablebases
            .probe_wdl(
                board.all_pieces_c(Color::White).0,
                board.all_pieces_c(Color::Black).0,
                board.bitboard_of_pt(PieceType::King).0,
                board.bitboard_of_pt(PieceType::Queen).0,
                board.bitboard_of_pt(PieceType::Rook).0,
                board.bitboard_of_pt(PieceType::Bishop).0,
                board.bitboard_of_pt(PieceType::Knight).0,
                board.bitboard_of_pt(PieceType::Pawn).0,
                board.ep_sq().map_or(0, |sq| sq as u32),
                board.ctm() == Color::White,
            )
            .ok()?;

        let stm_outcome = match wdl {
            WdlProbeResult::Win => 1,
            WdlProbeResult::Loss => -1,
            _ => 0,
        };
        Some(if board.ctm() == Color::White {
            stm_outcome
        } else {
            -stm_outcome
        })
    }

    fn relative_to(target: &Path, base: &Path) -> Option<PathBuf> {
        let target: Vec<_> = target.components().collect();
        let base: Vec<_> = base.components().collect();

        let common = target.iter().zip(&base).take_while(|(t, b)| t == b).count();
        // No common prefix means different drives; no relative path exists.
        if common == 0 {
            return None;
        }

        let mut rel = PathBuf::new();
        for _ in common..base.len() {
            rel.push("..");
        }
        rel.extend(&target[common..]);
        Some(rel)
    }
}

#[derive(Clone)]
struct Adapter;

impl EngineAdapter for Adapter {
    fn pawn_attacks(color: pyrrhic_rs::Color, sq: u64) -> u64 {
        let color = match color {
            pyrrhic_rs::Color::White => Color::White,
            pyrrhic_rs::Color::Black => Color::Black,
        };
        attacks::pawn_attacks_sq(SQ::from_repr(sq as u8), color).0
    }

    fn knight_attacks(sq: u64) -> u64 {
        attacks::knight_attacks(SQ::from_repr(sq as u8)).0
    }

    fn bishop_attacks(sq: u64, occ: u64) -> u64 {
        attacks::bishop_attacks(SQ::from_repr(sq as u8), Bitboard(occ)).0
    }

    fn rook_attacks(sq: u64, occ: u64) -> u64 {
        attacks::rook_attacks(SQ::from_repr(sq as u8), Bitboard(occ)).0
    }

    fn queen_attacks(sq: u64, occ: u64) -> u64 {
        Self::rook_attacks(sq, occ) | Self::bishop_attacks(sq, occ)
    }

    fn king_attacks(sq: u64) -> u64 {
        attacks::king_attacks(SQ::from_repr(sq as u8)).0
    }
}

struct Config {
    out: PathBuf,
    nodes: u64,
    threads: usize,
    positions: u64,
    syzygy: Option<String>,
    tb_men: u32,
    opening_plies: usize,
    rows_per_file: usize,
    hash_mb: usize,
}

impl Config {
    fn opt_number<T: FromStr>(caps: &Captures, name: &'static str) -> Result<Option<T>, String> {
        caps.name(name)
            .map(|m| {
                m.as_str()
                    .parse::<T>()
                    .map_err(|_| format!("Unable to parse {name}."))
            })
            .transpose()
    }

    fn parse(args: &[String]) -> Result<Self, String> {
        let line = args.join(" ");
        let caps = ARGS_RE
            .captures(&line)
            .ok_or("Unrecognized datagen arguments.")?;

        let defaults = Self::default();
        Ok(Self {
            out: caps
                .name("out")
                .map(|m| PathBuf::from(m.as_str()))
                .ok_or("--out is required.")?,
            nodes: Self::opt_number(&caps, "nodes")?.unwrap_or(defaults.nodes),
            threads: Self::opt_number(&caps, "threads")?
                .unwrap_or(defaults.threads)
                .max(1),
            positions: Self::opt_number(&caps, "positions")?.unwrap_or(defaults.positions),
            syzygy: caps.name("syzygy").map(|m| m.as_str().to_string()),
            tb_men: Self::opt_number(&caps, "tb_men")?.unwrap_or(defaults.tb_men),
            opening_plies: Self::opt_number(&caps, "opening_plies")?
                .unwrap_or(defaults.opening_plies),
            rows_per_file: Self::opt_number(&caps, "rows_per_file")?
                .unwrap_or(defaults.rows_per_file)
                .max(1),
            hash_mb: Self::opt_number(&caps, "hash")?.unwrap_or(defaults.hash_mb),
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            out: PathBuf::new(),
            nodes: 5000,
            threads: thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(1)),
            positions: u64::MAX,
            syzygy: None,
            tb_men: 5,
            opening_plies: 8,
            rows_per_file: 1_000_000,
            hash_mb: 16,
        }
    }
}

static ARGS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)^
                (?:
                    \s*--out\s+(?P<out>\S+) |
                    \s*--nodes\s+(?P<nodes>\d+) |
                    \s*--threads\s+(?P<threads>\d+) |
                    \s*--positions\s+(?P<positions>\d+) |
                    \s*--syzygy\s+(?P<syzygy>\S+) |
                    \s*--tb-men\s+(?P<tb_men>\d+) |
                    \s*--opening-plies\s+(?P<opening_plies>\d+) |
                    \s*--rows-per-file\s+(?P<rows_per_file>\d+) |
                    \s*--hash\s+(?P<hash>\d+)
                )*
            \s*$",
    )
    .expect("Datagen args regex should be valid.")
});

const USAGE: &str = "\
usage: weiawaga datagen --out DIR [options]
  --out DIR             output directory for parquet files (required)
  --nodes N             soft node limit per move (default 5000)
  --threads N           worker threads (default: cores - 1)
  --positions N         stop after roughly N recorded positions (default: run until killed)
  --syzygy PATH         syzygy tablebase directory for adjudication
  --tb-men N            probe positions with at most N pieces (default 5)
  --opening-plies N     random opening plies per game, N or N+1 at random (default 8)
  --rows-per-file N     rows per parquet file (default 1000000)
  --hash MB             per-thread transposition table size (default 16)";
