use crate::perft::perft::*;
use crate::search::search::*;
use crate::search::timer::*;
use crate::search::tt::*;
use crate::texel::tuner::*;
use crate::types::board::*;
use crate::uci::uci::*;
use crossbeam::thread;
use std::sync::atomic::*;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

// A lot of this nice uci implementation was inspired by Asymptote

pub struct SearchMaster {
    stop: Arc<AtomicBool>,
    num_threads: u16,
    tt: TT,
    overhead: Time,
}

impl SearchMaster {
    pub fn new(stop: Arc<AtomicBool>) -> Self {
        let search_defaults = SearchDefaults::default();
        Self {
            stop,
            num_threads: search_defaults.threads.0 as u16,
            tt: TT::new(search_defaults.hash.0 as usize),
            overhead: search_defaults.overhead.0 as Time,
        }
    }

    pub fn go(&mut self, board: &Board, time_control: TimeControl) {
        let mut main_search_thread = Search::new(
            Timer::new(board, time_control, self.stop.clone(), self.overhead),
            &self.tt,
            0,
        );

        let (best_move, _best_value) = thread::scope(|s| {
            for id in 1..self.num_threads {
                let mut helper_search_thread = Search::new(
                    Timer::new(
                        &board,
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
                    .spawn(move |_| helper_search_thread.go(board.clone()))
                    .unwrap();
            }
            main_search_thread.go(board.clone())
        })
        .unwrap();

        println!("bestmove {}", best_move.to_string());
        self.stop.store(false, Ordering::SeqCst);
        self.tt.clear();
    }

    pub fn run_loop(&mut self, rx: Receiver<UCICommand>) {
        let mut board = Board::new();

        for cmd in rx {
            match cmd {
                UCICommand::IsReady => {
                    println!("readyok");
                }
                UCICommand::UCINewGame => {
                    board = Board::new();
                }
                UCICommand::UCI => {
                    let default_options = SearchDefaults::default();
                    println!("id name Weiawaga v{}", env!("CARGO_PKG_VERSION"));
                    println!("id author {}", env!("CARGO_PKG_AUTHORS"));
                    println!(
                        "option name Hash type spin default {} min {} max {}",
                        default_options.hash.0, default_options.hash.1, default_options.hash.2
                    );
                    println!(
                        "option name Threads type spin default {} min {} max {}",
                        default_options.threads.0,
                        default_options.threads.1,
                        default_options.threads.2
                    );
                    println!(
                        "option name Overhead type spin default {} min {} max {}",
                        default_options.overhead.0,
                        default_options.overhead.1,
                        default_options.overhead.2
                    );
                    println!("uciok");
                }
                UCICommand::Position(new_board) => {
                    board = new_board;
                }
                UCICommand::Go(time_control) => {
                    self.stop.store(false, Ordering::SeqCst);
                    self.go(&board, time_control);
                }
                UCICommand::Perft(depth) => {
                    print_perft(&mut board, depth);
                }
                UCICommand::Option(name, value) => {
                    let search_defaults = SearchDefaults::default();
                    match name.as_ref() {
                        "Hash" => {
                            if let Ok(mut mb_size) = value.parse::<i128>() {
                                mb_size = mb_size
                                    .max(search_defaults.hash.1)
                                    .min(search_defaults.hash.2);
                                self.tt = TT::new(mb_size as usize);
                                println!("info string set Hash to {}MB", self.tt.mb_size());
                            } else {
                                eprintln!(
                                    "info string ERROR: error parsing Hash value; value remains at {}MB",
                                    self.tt.mb_size()
                                );
                            }
                        }
                        "Threads" => {
                            if let Ok(mut num_threads) = value.parse::<i128>() {
                                num_threads = num_threads
                                    .max(search_defaults.threads.1)
                                    .min(search_defaults.threads.2);
                                self.num_threads = num_threads as u16;
                                println!("info string set Threads to {}", self.num_threads);
                            } else {
                                eprintln!(
                                    "info string ERROR: error parsing Threads value; value remains at {}.",
                                    self.num_threads
                                );
                            }
                        }
                        "Overhead" => {
                            if let Ok(mut overhead) = value.parse::<i128>() {
                                overhead = overhead
                                    .max(search_defaults.overhead.1)
                                    .min(search_defaults.overhead.2);
                                self.overhead = overhead as Time;
                                println!("info string set Overhead to {}", self.overhead);
                            } else {
                                eprintln!(
                                    "info string ERROR: error parsing Overhead value; value remains at {}.",
                                    self.overhead
                                );
                            }
                        }
                        _ => {}
                    }
                }
                UCICommand::Tune(filename) => {
                    let mut tuner = Tuner::new(&filename);
                    tuner.tune();
                }
                UCICommand::Eval => {
                    println!("{}", board.eval());
                }
                _ => {
                    eprintln!("Unexpected UCI Command.");
                }
            }
        }
    }
}

struct SearchDefaults {
    // default, min, max
    hash: (i128, i128, i128),
    threads: (i128, i128, i128),
    overhead: (i128, i128, i128),
}

impl Default for SearchDefaults {
    fn default() -> Self {
        Self {
            hash: (16, 1, 128 * 1024),
            threads: (1, 1, 512),
            overhead: (0, 0, 5000),
        }
    }
}
