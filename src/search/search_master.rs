use crate::perft::perft::*;
use crate::search::search::*;
use crate::search::timer::*;
use crate::search::tt::*;
use crate::texel::tuner::*;
use crate::types::board::*;
use crate::uci::uci::*;
use crate::UCICommand;
use crossbeam::thread;
use std::sync::atomic::*;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

pub struct SearchMaster {
    stop: Arc<AtomicBool>,
    num_threads: u16,
    tt: TT,
}

impl SearchMaster {
    pub fn new(stop: Arc<AtomicBool>) -> Self {
        let options = get_options();
        let tt_size = if let UCIOption::Spin { default, .. } = options["Hash"] {
            default as u64
        } else {
            0
        };

        let num_threads = if let UCIOption::Spin { default, .. } = options["Threads"] {
            default as u16
        } else {
            1
        };

        Self {
            stop,
            num_threads,
            tt: TT::new(tt_size),
        }
    }

    pub fn go(&mut self, board: &Board, time_control: TimeControl) {
        let main_thread_timer = Timer::new(board, time_control, self.stop.clone());
        let mut main_search_thread = Search::new(main_thread_timer, &self.tt, 0);

        let (best_move, best_value) = thread::scope(|s| {
            for id in 1..self.num_threads {
                let helper_thread_timer =
                    Timer::new(&board, TimeControl::Infinite, self.stop.clone());

                let mut search_thread = Search::new(helper_thread_timer, &self.tt, id);
                s.builder()
                    .name(format!("Search thread #{:>3}", id))
                    .stack_size(8 * 1024 * 1024)
                    .spawn(move |_| search_thread.go(board.clone()))
                    .unwrap();
            }
            main_search_thread.go(board.clone())
        })
        .unwrap();

        println!("info score cp {}", best_value);
        println!("bestmove {}", best_move.to_string());
        self.stop.store(false, Ordering::SeqCst);
        self.tt.clear();
    }

    pub fn run_loop(&mut self, rx: Receiver<UCICommand>) {
        // global board
        let mut board = Board::new();
        let options = get_options();

        for cmd in rx {
            match cmd {
                UCICommand::IsReady => {
                    println!("readyok");
                }
                UCICommand::UCINewGame => {
                    board = Board::new();
                }
                UCICommand::Position(new_board, moves) => {
                    board = new_board;
                    for m in moves {
                        match board.push_str(m) {
                            Ok(_) => {}
                            Err(e) => {
                                board = new_board;
                                println!("{}", e);
                            }
                        }
                    }
                }
                UCICommand::Go(time_control) => {
                    self.go(&board, time_control);
                }
                UCICommand::Perft(depth) => {
                    print_perft(&mut board, depth);
                }
                UCICommand::Option(name, value) => match name.as_ref() {
                    "Hash" => {
                        if let Ok(mb_size) = value.parse::<u64>() {
                            self.tt = TT::new(mb_size);
                        }
                    }
                    "Threads" => {
                        if let Ok(num_threads) = value.parse::<u16>() {
                            self.num_threads = num_threads;
                        }
                    }
                    _ => {}
                },
                UCICommand::Tune(filename) => {
                    let mut tuner = Tuner::new(filename);
                    tuner.tune();
                }
                _ => {
                    println!("Unexpected UCI Command.");
                }
            }
        }
    }
}
