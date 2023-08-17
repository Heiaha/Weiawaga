use std::sync;
use std::sync::atomic::*;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

use super::board::*;
use super::perft::*;
use super::search::*;
use super::timer::*;
use super::tt::*;
use super::uci::*;

pub struct SearchMaster {
    stop: sync::Arc<AtomicBool>,
    board: Board,
    num_threads: u16,
    tt: TT,
    overhead: Duration,
}

impl SearchMaster {
    pub fn new(stop: sync::Arc<AtomicBool>) -> Self {
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
                }
                UCICommand::UCI => {
                    println!("id name Weiawaga v{}", env!("CARGO_PKG_VERSION"));
                    println!("id author {}", env!("CARGO_PKG_AUTHORS"));
                    println!("option name Hash type spin default 16 min 1 max 65536");
                    println!("option name Threads type spin default 1 min 1 max 512");
                    println!("option name Overhead type spin default 0 min 0 max 5000");
                    println!("uciok");
                }
                UCICommand::Position(fen, move_strs) => {
                    self.set_board(fen, move_strs);
                }
                UCICommand::Go(time_control) => {
                    self.go(time_control);
                }
                UCICommand::Perft(depth) => {
                    print_perft(&mut self.board, depth);
                }
                UCICommand::Option(name, value) => {
                    self.set_option(name, value);
                }
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

        let best_move = thread::scope(|s| {
            // Create main search thread with the actual time control. This thread controls self.stop.
            let mut main_search_thread = Search::new(
                Timer::new(&self.board, time_control, self.stop.clone(), self.overhead),
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

        self.tt.clear();
    }

    fn set_board(&mut self, fen: Option<String>, move_strs: Vec<String>) {
        let original_board = self.board.clone();
        if let Some(fen) = fen {
            if let Err(e) = self.board.set_fen(&fen) {
                eprintln!("{}", e);
                self.board = original_board;
                return;
            }
        } else {
            self.board.reset();
        }

        for move_str in move_strs {
            if let Err(e) = self.board.push_str(&move_str) {
                eprintln!("{}", e);
                self.board = original_board;
                return;
            }
        }
    }

    fn set_option(&mut self, name: String, value: String) {
        let result = match (name.as_ref(), value.parse::<u128>()) {
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
                eprintln!("info string ERROR: Option not recognized or parsing error.");
                return;
            }
        };

        println!("info string set {}", result);
    }
}
