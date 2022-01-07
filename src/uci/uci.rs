use crate::perft::perft::*;
use crate::search::search::*;
use crate::search::timer::*;
use crate::search::tt::*;
use crate::texel::tuner::*;
use crate::types::board::*;
use std::collections::HashMap;
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{io, sync, thread};

enum Option {
    Str { default: String },
    Spin { default: i32, min: i32, max: i32 },
    Check { default: bool },
    Combo { default: String },
    Button,
}

impl Option {
    fn string(default: &'static str) -> Self {
        Self::Str {
            default: String::from(default),
        }
    }

    fn spin(default: i32, min: i32, max: i32) -> Self {
        Self::Spin {
            default: default,
            min: min,
            max: max,
        }
    }
    fn check(default: bool) -> Self {
        Self::Check { default: default }
    }

    fn combo(default: &'static str) -> Self {
        Self::Combo {
            default: String::from(default),
        }
    }
}

fn get_options() -> HashMap<String, Option> {
    let mut opts = HashMap::new();
    opts.insert(String::from("Hash"), Option::spin(16, 1, 128 * 1024));
    opts
}

fn print_options() {
    // printing scheme from Rustfish
    let opts = get_options();
    for (name, opt) in opts.iter() {
        println!(
            "option name {} type {}",
            name,
            match opt {
                Option::Str { default, .. } => format!("string default {}", default),
                Option::Spin {
                    default, min, max, ..
                } => format!("spin default {} min {} max {}", default, min, max),
                Option::Check { default, .. } => format!("check default {}", default),
                Option::Button => format!("button"),
                Option::Combo { default, .. } => format!("combo default {}", default),
            }
        );
    }
}

pub enum UCICommand {
    Unknown(String),
    UCINewGame,
    UCI,
    IsReady,
    Position(Board, Vec<String>),
    Go(TimeControl),
    Quit,
    Stop,
    Perft(Depth),
    Option(String, String),
    Tune(String),
}

impl UCICommand {
    fn thread_loop(rec: sync::mpsc::Receiver<UCICommand>, abort: Arc<AtomicBool>) {
        // global board
        let mut board = Board::new();
        let options = get_options();

        // global transposition table
        let mut tt: TT = TT::new(
            if let Option::Spin { default, .. } = options["Hash"] {
                default as u64
            } else {
                0
            },
        );
        for cmd in rec {
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
                    let timer = Timer::new(&board, time_control, abort.clone());
                    let mut search = Search::new(timer, &mut tt);
                    let (best_move, best_score) = search.go(&mut board);
                    println!("info score cp {}", best_score);
                    println!("bestmove {}", best_move.to_string());
                    abort.store(false, Ordering::SeqCst);
                }
                UCICommand::Perft(depth) => {
                    print_perft(&mut board, depth);
                }
                UCICommand::Option(name, value) => match name.as_ref() {
                    "Hash" => {
                        if let Ok(mb) = value.parse::<u64>() {
                            tt.resize(mb);
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

    pub fn run() {
        println!("Weiawaga");
        println!("Homepage and source code: https://github.com/Heiaha/Weiawaga");
        let stdin = io::stdin();
        let lock = stdin.lock();

        let thread_moved_abort = sync::Arc::new(sync::atomic::AtomicBool::new(false));
        let abort = sync::Arc::clone(&thread_moved_abort);
        let (main_tx, main_rx) = sync::mpsc::channel();
        let handle = thread::spawn(move || Self::thread_loop(main_rx, thread_moved_abort));

        for line in lock.lines() {
            let cmd = UCICommand::from(&*line.unwrap());
            match cmd {
                UCICommand::Quit => return,
                UCICommand::Stop => abort.store(true, Ordering::SeqCst),
                UCICommand::UCI => {
                    println!("id name Weiawaga");
                    println!("id author Malarksist");
                    print_options();
                    println!("uciok");
                }
                cmd => main_tx.send(cmd).unwrap(),
            }
        }
    }
}

impl UCICommand {
    const HASH_DEFAULT: u64 = 512;
}

impl From<&str> for UCICommand {
    fn from(line: &str) -> Self {
        if line.starts_with("ucinewgame") {
            return UCICommand::UCINewGame;
        } else if line.starts_with("setoption") {
            let mut words = line.split_whitespace();
            let mut name_parts = Vec::new();
            let mut value_parts = Vec::new();

            // parse option name
            for word in words.by_ref() {
                if word == "value" {
                    break;
                } else {
                    name_parts.push(word);
                }
            }
            for word in words {
                value_parts.push(word);
            }
            let name = name_parts.last().unwrap();
            let value = value_parts.last().unwrap_or(&"");
            return UCICommand::Option(name.parse().unwrap(), value.parse().unwrap());
        } else if line.starts_with("uci") {
            return UCICommand::UCI;
        } else if line.starts_with("isready") {
            return UCICommand::IsReady;
        } else if line.starts_with("go") {
            let time_control = TimeControl::from(line);
            return UCICommand::Go(time_control);
        } else if line.starts_with("position") {
            let pos;
            let fen = line.trim_start_matches("position ");
            if fen.starts_with("startpos") {
                pos = Board::new();
            } else {
                pos = Board::from(fen.trim_start_matches("fen"));
            }

            let mut moves = Vec::new();
            if line.contains("moves") {
                if let Some(moves_str) = line.split_terminator("moves ").nth(1) {
                    for mov in moves_str.split_whitespace() {
                        moves.push(String::from(mov));
                    }
                }
            }
            return UCICommand::Position(pos, moves);
        } else if line.starts_with("quit") {
            return UCICommand::Quit;
        } else if line.starts_with("perft") {
            let depth = line
                .split_whitespace()
                .nth(1)
                .and_then(|d| d.parse().ok())
                .unwrap_or(6);
            return UCICommand::Perft(depth);
        } else if line == "stop" {
            return UCICommand::Stop;
        } else if line.starts_with("tune") {
            let filename = line.split_whitespace().nth(1).unwrap();
            return UCICommand::Tune(filename.parse().unwrap());
        }
        Self::Unknown(line.to_owned())
    }
}
