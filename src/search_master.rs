use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::board::*;
use super::perft::*;
use super::search::*;
use super::timer::*;
use super::tt::*;
use super::uci::*;

pub struct SearchMaster {
    stop: Arc<AtomicBool>,
    board: Board,
    num_threads: u16,
    tt: TT,
    overhead: Duration,
}

impl SearchMaster {
    pub fn new(stop: Arc<AtomicBool>) -> Self {
        Self {
            stop,
            board: Board::new(),
            num_threads: 1,
            tt: TT::new(16),
            overhead: Duration::ZERO,
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
                    println!("option name Overhead type spin default 0 min 0 max 5000");
                    println!("uciok");
                }
                UCICommand::Position { fen, moves } => {
                    match self.set_board(fen, moves) {
                        Ok(_) => (),
                        Err(err) => eprintln!("{}", err),
                    };
                }
                UCICommand::Go(time_control) => {
                    self.go(time_control);
                }
                UCICommand::Perft(depth) => {
                    print_perft(&mut self.board, depth);
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
        }
    }

    fn go(&mut self, time_control: TimeControl) {
        self.stop.store(false, Ordering::SeqCst);
        let nodes = Arc::new(AtomicU64::new(0));

        let best_move = thread::scope(|s| {
            // Create main search thread with the actual time control. This thread controls self.stop.
            let mut main_search_thread = Search::new(
                Timer::new(
                    &self.board,
                    time_control,
                    self.stop.clone(),
                    nodes.clone(),
                    self.overhead,
                ),
                &self.tt,
                0,
            );

            // Create helper search threads which will stop when self.stop resolves to true.
            for id in 1..self.num_threads {
                let thread_board = self.board.clone();
                let mut helper_search_thread = Search::new(
                    Timer::new(
                        &thread_board,
                        TimeControl::Infinite,
                        self.stop.clone(),
                        nodes.clone(),
                        self.overhead,
                    ),
                    &self.tt,
                    id,
                );
                s.spawn(move || helper_search_thread.go(thread_board));
            }
            main_search_thread.go(self.board.clone())
        });

        match best_move {
            Some(m) => println!("bestmove {}", m),
            None => println!("bestmove (none)"),
        }
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
        let result = match (name.as_str(), value.parse::<u128>()) {
            ("Hash", Ok(parsed_value)) => {
                self.tt = TT::new(parsed_value as usize);
                format!("Hash to {}MB", self.tt.mb_size())
            }
            ("Threads", Ok(parsed_value)) => {
                self.num_threads = parsed_value as u16;
                format!("Threads to {}", self.num_threads)
            }
            ("Overhead", Ok(parsed_value)) => {
                self.overhead = Duration::from_millis(parsed_value as u64);
                format!("Overhead to {}ms", self.overhead.as_millis())
            }
            _ => {
                return Err(());
            }
        };

        Ok(result)
    }
}
