use crate::search::search::*;
use crate::search::search_master::*;
use crate::search::timer::*;
use crate::types::board::*;
use std::collections::HashMap;
use std::io::BufRead;
use std::sync::atomic::Ordering;
use std::{io, sync, thread};

pub enum UCIOption {
    Str { default: String },
    Spin { default: i32, min: i32, max: i32 },
    Check { default: bool },
    Combo { default: String },
    Button,
}

impl UCIOption {
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
    pub fn run() {
        println!("Weiawaga");
        println!("Homepage and source code: https://github.com/Heiaha/Weiawaga");
        let stdin = io::stdin();
        let lock = stdin.lock();

        let thread_abort = sync::Arc::new(sync::atomic::AtomicBool::new(false));
        let abort = sync::Arc::clone(&thread_abort);
        let (main_tx, main_rx) = sync::mpsc::channel();

        let handle = thread::spawn(move || SearchMaster::new(abort).run_loop(main_rx));

        for line in lock.lines() {
            let cmd = UCICommand::from(&*line.unwrap());
            match cmd {
                UCICommand::Quit => return,
                UCICommand::Stop => thread_abort.store(true, Ordering::SeqCst),
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

pub fn get_options() -> HashMap<String, UCIOption> {
    let mut opts = HashMap::new();
    opts.insert(String::from("Hash"), UCIOption::spin(16, 1, 128 * 1024));
    opts.insert(String::from("Threads"), UCIOption::spin(1, 1, 512));
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
                UCIOption::Str { default, .. } => format!("string default {}", default),
                UCIOption::Spin {
                    default, min, max, ..
                } => format!("spin default {} min {} max {}", default, min, max),
                UCIOption::Check { default, .. } => format!("check default {}", default),
                UCIOption::Button => format!("button"),
                UCIOption::Combo { default, .. } => format!("combo default {}", default),
            }
        );
    }
}
