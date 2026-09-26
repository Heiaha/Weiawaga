use super::board::*;
use super::move_sorting::*;
#[cfg(feature = "tune")]
use super::params::{self, Tunable};
use super::perft::*;
use super::search::*;
use super::timer::*;
use super::tt::*;
use super::uci::*;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

pub struct SearchMaster {
    stop: Arc<AtomicBool>,
    pondering: Arc<AtomicBool>,
    ponder_enabled: bool,
    show_wdl: bool,
    multi_pv: usize,
    board: Board,
    tt: TT,
    scorers: Vec<MoveScorer>,
    overhead: Duration,
}

impl SearchMaster {
    pub fn new(stop: Arc<AtomicBool>, pondering: Arc<AtomicBool>) -> Self {
        Self {
            stop,
            pondering,
            ponder_enabled: false,
            show_wdl: false,
            multi_pv: EngineOption::MULTIPV_DEFAULT,
            board: Board::new(),
            tt: TT::new(16),
            scorers: vec![MoveScorer::new()],
            overhead: Duration::from_millis(10),
        }
    }

    pub fn run(&mut self, main_rx: Receiver<UCICommand>) {
        for cmd in main_rx {
            match cmd {
                UCICommand::IsReady => {
                    println!("readyok");
                }
                UCICommand::UCINewGame => {
                    self.board.reset();
                    self.tt.clear();
                    self.scorers.fill_with(MoveScorer::new);
                }
                UCICommand::UCI => {
                    println!("id name Weiawaga v{}", env!("CARGO_PKG_VERSION"));
                    println!("id author {}", env!("CARGO_PKG_AUTHORS"));
                    println!(
                        "option name Hash type spin default {} min {} max {}",
                        EngineOption::HASH_DEFAULT,
                        EngineOption::HASH_MIN,
                        EngineOption::HASH_MAX,
                    );
                    println!(
                        "option name Threads type spin default {} min {} max {}",
                        EngineOption::THREADS_DEFAULT,
                        EngineOption::THREADS_MIN,
                        EngineOption::THREADS_MAX,
                    );
                    println!(
                        "option name Move Overhead type spin default {} min {} max {}",
                        EngineOption::MOVE_OVERHEAD_DEFAULT.as_millis(),
                        EngineOption::MOVE_OVERHEAD_MIN.as_millis(),
                        EngineOption::MOVE_OVERHEAD_MAX.as_millis(),
                    );
                    println!(
                        "option name Ponder type check default {}",
                        EngineOption::PONDER_DEFAULT,
                    );
                    println!(
                        "option name UCI_ShowWDL type check default {}",
                        EngineOption::SHOW_WDL_DEFAULT,
                    );
                    println!(
                        "option name MultiPV type spin default {} min {} max {}",
                        EngineOption::MULTIPV_DEFAULT,
                        EngineOption::MULTIPV_MIN,
                        EngineOption::MULTIPV_MAX,
                    );
                    println!("option name Clear Hash type button");
                    #[cfg(feature = "tune")]
                    for &Tunable {
                        name,
                        default,
                        min,
                        max,
                    } in params::OPTIONS
                    {
                        println!(
                            "option name {name} type spin default {default} min {min} max {max}"
                        );
                    }
                    println!("uciok");
                }
                UCICommand::Position(board) => self.board = *board,
                UCICommand::Go {
                    time_control,
                    ponder,
                    searchmoves,
                } => self.go(time_control, ponder, searchmoves),
                UCICommand::Perft(depth) => {
                    let mut board = self.board.clone();
                    print_perft(&mut board, depth);
                }
                UCICommand::Option(engine_option) => match self.set_option(engine_option) {
                    Ok(_) => (),
                    Err(e) => eprintln!("{e}"),
                },
                UCICommand::Eval => println!("{}", self.board.eval()),
                UCICommand::Fen => println!("{}", self.board),
                _ => eprintln!("Unexpected UCI Command."),
            }
            std::io::stdout().flush().unwrap();
            std::io::stderr().flush().unwrap();
        }
    }

    fn go(&mut self, time_control: TimeControl, ponder: bool, searchmoves: Vec<String>) {
        if ponder && !self.ponder_enabled {
            eprintln!("Pondering is not enabled.");
            return;
        }

        self.pondering.store(ponder, Ordering::Release);
        self.stop.store(false, Ordering::Release);
        let options = SearchOptions {
            show_wdl: self.show_wdl,
            multi_pv: self.multi_pv,
            searchmoves,
        };
        let signals = Signals::new(
            self.stop.clone(),
            self.pondering.clone(),
            self.scorers.len(),
        );

        ///////////////////////////////////////////////////////////////////
        // Only the main thread keeps the clock and reports the requested
        // lines. Helpers search until it stops them and only feed the tt.
        ///////////////////////////////////////////////////////////////////
        let mut searches = self
            .scorers
            .iter_mut()
            .enumerate()
            .map(|(id, scorer)| {
                scorer.clear_killers();
                let (control, options) = if id == 0 {
                    (time_control, options.clone())
                } else {
                    let options = SearchOptions {
                        multi_pv: EngineOption::MULTIPV_DEFAULT,
                        ..options.clone()
                    };
                    (TimeControl::Infinite, options)
                };
                let timer = Timer::new(&self.board, control, signals.clone(), id, self.overhead);
                Search::new(id as u16, timer, &self.tt, scorer, options)
            })
            .collect::<Vec<_>>();

        let (main, helpers) = searches
            .split_first_mut()
            .expect("There is always at least one search thread.");
        let (best_move, ponder_move) = thread::scope(|s| {
            for helper in helpers {
                let board = self.board.clone();
                s.spawn(move || helper.go(board));
            }
            main.go(self.board.clone())
        });

        // UCI forbids sending bestmove while in ponder mode; if the search
        // ended on its own, hold the reply until ponderhit or stop arrives.
        while self.pondering.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(1));
        }

        match (best_move, ponder_move) {
            (Some(best), Some(ponder)) if self.ponder_enabled => {
                println!("bestmove {best} ponder {ponder}")
            }
            (Some(best), _) => println!("bestmove {best}"),
            (None, _) => println!("bestmove (none)"),
        }
        self.tt.age_up();
    }

    fn checked<T: PartialOrd>(
        value: T,
        range: std::ops::RangeInclusive<T>,
        err: &'static str,
    ) -> Result<T, &'static str> {
        range.contains(&value).then_some(value).ok_or(err)
    }

    fn set_option(&mut self, engine_option: EngineOption) -> Result<(), &'static str> {
        match engine_option {
            EngineOption::Hash(mb) => {
                self.tt = TT::new(Self::checked(
                    mb,
                    EngineOption::HASH_MIN..=EngineOption::HASH_MAX,
                    "Hash size out of range.",
                )?);
            }
            EngineOption::Threads(n_threads) => {
                let n_threads = Self::checked(
                    n_threads,
                    EngineOption::THREADS_MIN..=EngineOption::THREADS_MAX,
                    "Threads out of range.",
                )?;
                self.scorers
                    .resize_with(usize::from(n_threads), MoveScorer::new);
            }
            EngineOption::MoveOverhead(overhead) => {
                self.overhead = Self::checked(
                    overhead,
                    EngineOption::MOVE_OVERHEAD_MIN..=EngineOption::MOVE_OVERHEAD_MAX,
                    "Move overhead out of range.",
                )?;
            }
            EngineOption::Ponder(ponder_enabled) => {
                self.ponder_enabled = ponder_enabled;
            }
            EngineOption::ShowWDL(show_wdl) => {
                self.show_wdl = show_wdl;
            }
            EngineOption::MultiPV(multi_pv) => {
                self.multi_pv = Self::checked(
                    multi_pv,
                    EngineOption::MULTIPV_MIN..=EngineOption::MULTIPV_MAX,
                    "MultiPV out of range.",
                )?;
            }
            EngineOption::ClearHash => {
                self.tt.clear();
            }
            #[cfg(feature = "tune")]
            EngineOption::Tunable(name, value) => {
                params::set(&name, value)?;
                Search::rebuild_lmr_table();
            }
        };
        Ok(())
    }
}
