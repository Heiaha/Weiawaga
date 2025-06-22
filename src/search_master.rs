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
    num_threads: u16,
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
            num_threads: 1,
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
                    println!("option name Hash type spin default 16 min 1 max 65536");
                    println!("option name Threads type spin default 1 min 1 max 512");
                    println!("option name MoveOverhead type spin default 10 min 0 max 5000");
                    println!("option name Ponder type check default false");
                    println!("uciok");
                }
                UCICommand::Position { fen, moves } => {
                    match self.set_board(fen, moves) {
                        Ok(_) => (),
                        Err(err) => eprintln!("{}", err),
                    };
                }
                UCICommand::Go {
                    time_control,
                    ponder,
                } => {
                    self.go(time_control, ponder);
                }
                UCICommand::Perft(depth) => {
                    let mut board = self.board.duplicate();
                    print_perft(&mut board, depth);
                }
                UCICommand::Option { name, value } => match self.set_option(name, value) {
                    Ok(result) => println!("info string set {}", result),
                    Err(_) => eprintln!("Option not recognized or parsing error."),
                },
                UCICommand::Eval => {
                    println!("{}", self.board.eval());
                }
                UCICommand::Fen => {
                    println!("{}", self.board);
                }
                _ => {
                    eprintln!("Unexpected UCI Command.");
                }
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

        let board = self.board.duplicate();

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
            for id in 1..self.num_threads {
                let thread_board = board.duplicate();
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
            main_search_thread.go(board.duplicate())
        });

        match (best_move, ponder_move) {
            (Some(best), Some(ponder)) if self.ponder_enabled => {
                println!("bestmove {} ponder {}", best, ponder)
            }
            (Some(best), _) => println!("bestmove {}", best),
            (None, _) => println!("bestmove (none)"),
        }
        self.tt.age_up();
    }

    fn set_board(&mut self, fen: Option<String>, moves: Vec<String>) -> Result<(), &str> {
        let mut board = Board::new();
        if let Some(fen) = fen {
            board.set_fen(&fen)?;
        }

        for m in moves {
            board.push_str(&m)?;
        }

        self.board = board;
        Ok(())
    }

    fn set_option(&mut self, name: String, value: String) -> Result<String, ()> {
        let result = match name.as_str() {
            "Hash" => {
                let mb = value.parse::<usize>().map_err(|_| ())?;
                self.tt = TT::new(mb);
                format!("Hash to {}MB", self.tt.mb_size())
            }
            "Threads" => {
                self.num_threads = value.parse::<u16>().map_err(|_| ())?;
                format!("Threads to {}", self.num_threads)
            }
            "MoveOverhead" => {
                let ms = value.parse::<u64>().map_err(|_| ())?;
                self.overhead = Duration::from_millis(ms);
                format!("MoveOverhead to {}ms", self.overhead.as_millis())
            }
            "Ponder" => {
                let enabled = match value.trim().to_ascii_lowercase().as_str() {
                    "true" | "on" | "1" => Ok(true),
                    "false" | "off" | "0" => Ok(false),
                    _ => Err(()),
                }?;
                self.ponder_enabled = enabled;
                format!("Ponder {}", if enabled { "on" } else { "off" })
            }
            _ => {
                return Err(());
            }
        };

        Ok(result)
    }
}
