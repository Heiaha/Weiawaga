use std::sync;
use std::sync::atomic::*;
use std::sync::mpsc::Receiver;
use std::thread;

use super::board::*;
use super::perft::*;
use super::search::*;
use super::timer::*;
use super::tt::*;
use super::types::*;
use super::uci::*;

pub struct SearchMaster {
    stop: sync::Arc<AtomicBool>,
    board: Board,
    num_threads: u16,
    tt: TT,
    overhead: Time,
}

impl SearchMaster {
    pub fn new(stop: sync::Arc<AtomicBool>) -> Self {
        Self {
            stop,
            board: Board::new(),
            num_threads: 1,
            tt: TT::new(16),
            overhead: 0,
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
                    println!("option name Move Overhead type spin default 0 min 0 max 5000");
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
                _ => {
                    eprintln!("Unexpected UCI Command.");
                }
            }
        }
    }

    pub fn go(&mut self, time_control: TimeControl) {
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
        match name.as_ref() {
            "Hash" => {
                if let Ok(mb_size) = value.parse() {
                    self.tt = TT::new(mb_size);
                    println!("info string set Hash to {}MB", self.tt.mb_size());
                } else {
                    eprintln!(
                        "info string ERROR: error parsing Hash value; value remains at {}MB",
                        self.tt.mb_size()
                    );
                }
            }
            "Threads" => {
                if let Ok(num_threads) = value.parse() {
                    self.num_threads = num_threads;
                    println!("info string set Threads to {}", self.num_threads);
                } else {
                    eprintln!(
                        "info string ERROR: error parsing Threads value; value remains at {}.",
                        self.num_threads
                    );
                }
            }
            "Move Overhead" => {
                if let Ok(overhead) = value.parse() {
                    self.overhead = overhead;
                    println!("info string set Move Overhead to {}", self.overhead);
                } else {
                    eprintln!(
                        "info string ERROR: error parsing Move Overhead value; value remains at {}.",
                        self.overhead
                    );
                }
            }
            _ => {
                eprintln!("Option not recognized.")
            }
        }
    }
}
