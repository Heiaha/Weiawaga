use std::sync;
use std::sync::atomic::*;
use std::sync::mpsc::Receiver;

use crossbeam::thread;

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
                    self.board = Board::new();
                }
                UCICommand::UCI => {
                    println!("id name Weiawaga v{}", env!("CARGO_PKG_VERSION"));
                    println!("id author {}", env!("CARGO_PKG_AUTHORS"));
                    println!("option name Hash type spin default 16 min 1 max 65536");
                    println!("option name Threads type spin default 1 min 1 max 512");
                    println!("option name Overhead type spin default 0 min 0 max 5000");
                    println!("uciok");
                }
                UCICommand::Position(new_board) => {
                    self.board = new_board;
                }
                UCICommand::Go(time_control) => {
                    self.go(time_control);
                }
                UCICommand::Perft(depth) => {
                    print_perft(&mut self.board, depth);
                }
                UCICommand::Option(name, value) => {
                    self.set_option(&name, &value);
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
                s.builder()
                    .name(format!("Search thread #{:>3}", id))
                    .stack_size(8 * 1024 * 1024)
                    .spawn(move |_| helper_search_thread.go(thread_board))
                    .unwrap();
            }
            main_search_thread.go(self.board.clone())
        })
        .unwrap();
        println!("bestmove {}", best_move);
        self.tt.clear();
    }

    fn set_option(&mut self, name: &str, value: &str) {
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
            "Overhead" => {
                if let Ok(overhead) = value.parse() {
                    self.overhead = overhead;
                    println!("info string set Overhead to {}", self.overhead);
                } else {
                    eprintln!(
                        "info string ERROR: error parsing Overhead value; value remains at {}.",
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
