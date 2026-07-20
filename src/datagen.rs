use std::fs::{self, File};
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
use rand::Rng;
use regex::{Captures, Regex};

use super::board::*;
use super::moov::*;
use super::move_list::*;
use super::move_sorting::*;
use super::piece::*;
use super::search::*;
use super::timer::*;
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
    book: Vec<String>,
    produced: AtomicU64,
}

impl DataGen {
    fn new(cfg: Config) -> Result<Self, String> {
        let book = cfg
            .book
            .as_deref()
            .map(Self::load_book)
            .transpose()?
            .unwrap_or_default();

        if !book.is_empty() {
            println!("Loaded {} book positions.", book.len());
        }

        Ok(Self {
            cfg,
            book,
            produced: AtomicU64::new(0),
        })
    }

    // EPD or FEN, one position per line: the first four fields are the
    // position; any move counters or epd opcodes are dropped and reset.
    fn load_book(path: &Path) -> Result<Vec<String>, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Unable to read the book: {e}"))?;

        let book: Vec<String> = content
            .lines()
            .filter_map(|line| {
                let fields: Vec<&str> = line.split_whitespace().take(4).collect();
                (fields.len() == 4).then(|| format!("{} 0 1", fields.join(" ")))
            })
            .collect();

        // Individual bad lines are skipped at play time, but a first line
        // that doesn't parse means the whole file is the wrong format.
        let first = book
            .first()
            .ok_or_else(|| format!("No positions found in {}.", path.display()))?;
        if Board::try_from(first.as_str()).is_err() {
            return Err(format!(
                "{} does not look like an epd/fen book.",
                path.display()
            ));
        }
        Ok(book)
    }

    fn run(&self) {
        fs::create_dir_all(&self.cfg.out).expect("Unable to create the output directory.");

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
        let mut board = self.start_position(rng)?;
        let nudge_plies = self.pick_nudge_plies(board.ply(), rng);

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

            let quiet_spot = !board.in_check() && m.is_quiet();
            let sample =
                quiet_spot.then(|| (board.to_string(), white_value.clamp(-16000, 16000) as i16));

            // The sample keeps the searched eval, so a nudge below doesn't
            // touch the label; only the move played changes.
            let m = if quiet_spot && nudge_plies.contains(&board.ply()) {
                self.pick_nudge(&mut board, rng).unwrap_or(m)
            } else {
                m
            };

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

    fn pick_nudge_plies(&self, first_ply: usize, rng: &mut impl Rng) -> Vec<usize> {
        let window = self.cfg.nudge_max_ply.saturating_sub(first_ply);
        rand::seq::index::sample(rng, window, self.cfg.nudges.min(window))
            .iter()
            .map(|offset| first_ply + offset)
            .collect()
    }

    fn pick_nudge(&self, board: &mut Board, rng: &mut impl Rng) -> Option<Move> {
        let mut scored = Vec::new();
        let moves = MoveList::from::<false>(board);
        for &m in &moves {
            if !m.is_quiet() {
                continue;
            }
            board.push(m);
            if !board.in_check() && !Self::is_loud(board) {
                // The head answers for the side to move — here the opponent —
                // so its loss slot is the mover's win.
                let [win, draw, _] = board.wdl();
                scored.push((m, win + draw / 2.0));
            }
            board.pop();
        }

        let best = scored.iter().fold(f32::MIN, |acc, &(_, e)| acc.max(e));
        let candidates: Vec<Move> = scored
            .into_iter()
            .filter_map(|(m, e)| (best - e <= self.cfg.nudge_margin).then_some(m))
            .collect();

        (!candidates.is_empty()).then(|| candidates[rng.random_range(0..candidates.len())])
    }

    // The wdl head only ever trains on positions with no tactic pending (the
    // sample guard requires the searched best move to be quiet), so it must
    // not be consulted where the mover has a material-winning capture.
    fn is_loud(board: &Board) -> bool {
        let captures = MoveList::from::<true>(board);
        (&captures)
            .into_iter()
            .any(|&c| MoveScorer::see(board, c, 1))
    }

    fn start_position(&self, rng: &mut impl Rng) -> Option<Board> {
        if !self.book.is_empty() {
            let fen = &self.book[rng.random_range(0..self.book.len())];
            return Board::try_from(fen.as_str()).ok();
        }

        let mut board = Board::new();
        // Half the games get one extra random ply so that each color is
        // equally often first to move out of the opening; a fixed even
        // count skews outcomes heavily toward white.
        for _ in 0..self.cfg.opening_plies + rng.random_range(0..=1) {
            let moves = MoveList::from::<false>(&board);
            if moves.len() == 0 {
                return None;
            }
            board.push(moves[rng.random_range(0..moves.len())]);
        }
        Some(board)
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

struct Config {
    out: PathBuf,
    nodes: u64,
    threads: usize,
    positions: u64,
    book: Option<PathBuf>,
    opening_plies: usize,
    nudges: usize,
    nudge_max_ply: usize,
    nudge_margin: f32,
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

    fn opt_path(caps: &Captures, name: &str) -> Option<PathBuf> {
        caps.name(name).map(|m| PathBuf::from(m.as_str()))
    }

    fn parse(args: &[String]) -> Result<Self, String> {
        let line = args.join(" ");
        let caps = ARGS_RE
            .captures(&line)
            .ok_or("Unrecognized datagen arguments.")?;

        let defaults = Self::default();
        Ok(Self {
            out: Self::opt_path(&caps, "out").ok_or("--out is required.")?,
            nodes: Self::opt_number(&caps, "nodes")?.unwrap_or(defaults.nodes),
            threads: Self::opt_number(&caps, "threads")?
                .unwrap_or(defaults.threads)
                .max(1),
            positions: Self::opt_number(&caps, "positions")?.unwrap_or(defaults.positions),
            book: Self::opt_path(&caps, "book"),
            opening_plies: Self::opt_number(&caps, "opening_plies")?
                .unwrap_or(defaults.opening_plies),
            nudges: Self::opt_number(&caps, "nudges")?.unwrap_or(defaults.nudges),
            nudge_max_ply: Self::opt_number(&caps, "nudge_max_ply")?
                .unwrap_or(defaults.nudge_max_ply),
            nudge_margin: Self::opt_number(&caps, "nudge_margin")?.unwrap_or(defaults.nudge_margin),
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
            book: None,
            opening_plies: 8,
            nudges: 5,
            nudge_max_ply: 24,
            nudge_margin: 0.1,
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
                    \s*--book\s+(?P<book>\S+) |
                    \s*--opening-plies\s+(?P<opening_plies>\d+) |
                    \s*--nudges\s+(?P<nudges>\d+) |
                    \s*--nudge-max-ply\s+(?P<nudge_max_ply>\d+) |
                    \s*--nudge-margin\s+(?P<nudge_margin>[\d.]+) |
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
  --book PATH           epd/fen file of start positions, one per line
  --opening-plies N     without --book: random opening plies per game, N or N+1 at random (default 8)
  --nudges N            plies per game where a random near-best quiet move is played (default 5)
  --nudge-max-ply N     nudges happen before this ply (default 24)
  --nudge-margin E      wdl expected-score drop (0-1) a nudge may accept (default 0.1)
  --rows-per-file N     rows per parquet file (default 1000000)
  --hash MB             per-thread transposition table size (default 16)";
