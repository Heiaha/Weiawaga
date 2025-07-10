use super::board::*;
use super::perft::*;
use super::search::*;
use super::timer::*;
use super::tt::*;
use super::uci::*;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

pub struct SearchMaster {
    stop: Arc<AtomicBool>,
    pondering: Arc<AtomicBool>,
    ponder_enabled: bool,
    board: Board,
    n_threads: u16,
    tt: TT,
    overhead: Duration,
}

impl SearchMaster {
    pub fn new(stop: Arc<AtomicBool>, pondering: Arc<AtomicBool>) -> Self {
        Self {
            stop,
            pondering,
            ponder_enabled: false,
            board: Board::new(),
            n_threads: 1,
            tt: TT::new(16),
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
                    println!("option name Clear Hash type button");
                    println!("uciok");
                }
                UCICommand::Position(board) => self.board = *board,
                UCICommand::Go {
                    time_control,
                    ponder,
                } => self.go(time_control, ponder),
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

    fn go(&mut self, time_control: TimeControl, ponder: bool) {
        if ponder && !self.ponder_enabled {
            eprintln!("Pondering is not enabled.");
            return;
        }

        let board = self.board.clone();

        self.pondering.store(ponder, Ordering::Release);
        self.stop.store(false, Ordering::Release);
        let nodes = Arc::new(AtomicU64::new(0));

        let (best_move, ponder_move) = thread::scope(|s| {
            // Create main search thread with the actual time control. This thread controls self.stop.
            let mut main_search_thread = Search::new(
                Timer::new(
                    &board,
                    time_control,
                    self.pondering.clone(),
                    self.stop.clone(),
                    nodes.clone(),
                    self.overhead,
                ),
                &self.tt,
                0,
            );

            // Create helper search threads which will stop when self.stop resolves to true.
            for id in 1..self.n_threads {
                let thread_board = board.clone();
                let mut helper_search_thread = Search::new(
                    Timer::new(
                        &thread_board,
                        TimeControl::Infinite,
                        self.pondering.clone(),
                        self.stop.clone(),
                        nodes.clone(),
                        self.overhead,
                    ),
                    &self.tt,
                    id,
                );
                s.spawn(move || helper_search_thread.go(thread_board));
            }
            main_search_thread.go(board.clone())
        });

        match (best_move, ponder_move) {
            (Some(best), Some(ponder)) if self.ponder_enabled => {
                println!("bestmove {best} ponder {ponder}")
            }
            (Some(best), _) => println!("bestmove {best}"),
            (None, _) => println!("bestmove (none)"),
        }
        self.tt.age_up();
    }

    fn set_option(&mut self, engine_option: EngineOption) -> Result<(), &'static str> {
        match engine_option {
            EngineOption::Hash(mb) => {
                if !(EngineOption::HASH_MIN..=EngineOption::HASH_MAX).contains(&mb) {
                    return Err("Hash size out of range.");
                }
                self.tt = TT::new(mb);
            }
            EngineOption::Threads(n_threads) => {
                if !(EngineOption::THREADS_MIN..=EngineOption::THREADS_MAX).contains(&n_threads) {
                    return Err("Threads out of range.");
                }
                self.n_threads = n_threads;
            }
            EngineOption::MoveOverhead(overhead) => {
                if !(EngineOption::MOVE_OVERHEAD_MIN..=EngineOption::MOVE_OVERHEAD_MAX)
                    .contains(&overhead)
                {
                    return Err("Hash size out of range.");
                }
                self.overhead = overhead;
            }
            EngineOption::Ponder(ponder_enabled) => {
                self.ponder_enabled = ponder_enabled;
            }
            EngineOption::ClearHash => {
                self.tt.clear();
            }
        };
        Ok(())
    }
}
