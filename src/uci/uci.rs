use crate::search::search::*;
use crate::search::timer::*;
use crate::types::board::*;
use crate::uci::search_master::*;
use std::collections::HashMap;
use std::io::BufRead;
use std::sync::atomic::Ordering;
use std::{io, sync, thread};

#[derive(Debug)]
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
        Self::Spin { default, min, max }
    }
    fn check(default: bool) -> Self {
        Self::Check { default }
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
        println!("Weiawaga v{}", env!("CARGO_PKG_VERSION"));
        println!("{}", env!("CARGO_PKG_REPOSITORY"));
        let stdin = io::stdin();
        let lock = stdin.lock();

        let thread_abort = sync::Arc::new(sync::atomic::AtomicBool::new(false));
        let abort = thread_abort.clone();
        let (main_tx, main_rx) = sync::mpsc::channel();

        let _handle = thread::spawn(move || SearchMaster::new(abort).run_loop(main_rx));

        for line in lock.lines() {
            let cmd = UCICommand::try_from(&*line.unwrap());
            match cmd {
                Ok(cmd) => match cmd {
                    UCICommand::Quit => return,
                    UCICommand::Stop => thread_abort.store(true, Ordering::SeqCst),
                    UCICommand::UCI => {
                        println!("id name Weiawaga v{}", env!("CARGO_PKG_VERSION"));
                        println!("id author {}", env!("CARGO_PKG_AUTHORS"));
                        print_options();
                        println!("uciok");
                    }
                    cmd => main_tx.send(cmd).unwrap(),
                },
                Err(e) => {
                    println!("{}", e);
                }
            }
        }
    }
}

impl TryFrom<&str> for UCICommand {
    type Error = &'static str;

    fn try_from(line: &str) -> Result<Self, Self::Error> {
        if line.starts_with("ucinewgame") {
            return Ok(UCICommand::UCINewGame);
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
            return Ok(UCICommand::Option(
                name.parse().unwrap(),
                value.parse().unwrap(),
            ));
        } else if line.starts_with("uci") {
            return Ok(UCICommand::UCI);
        } else if line.starts_with("isready") {
            return Ok(UCICommand::IsReady);
        } else if line.starts_with("go") {
            let time_control = TimeControl::from(line);
            return Ok(UCICommand::Go(time_control));
        } else if line.starts_with("position") {
            let pos;
            let fen = line.trim_start_matches("position ");
            if fen.starts_with("startpos") {
                pos = Board::new();
            } else {
                pos = Board::try_from(fen.trim_start_matches("fen"))?;
            }

            let mut moves = Vec::new();
            if line.contains("moves") {
                if let Some(moves_str) = line.split_terminator("moves ").nth(1) {
                    for mov in moves_str.split_whitespace() {
                        moves.push(String::from(mov));
                    }
                }
            }
            return Ok(UCICommand::Position(pos, moves));
        } else if line.starts_with("quit") {
            return Ok(UCICommand::Quit);
        } else if line.starts_with("perft") {
            let depth = line
                .split_whitespace()
                .nth(1)
                .and_then(|d| d.parse().ok())
                .unwrap_or(6);
            return Ok(UCICommand::Perft(depth));
        } else if line == "stop" {
            return Ok(UCICommand::Stop);
        } else if line.starts_with("tune") {
            let filename = line.split_whitespace().nth(1).unwrap();
            return Ok(UCICommand::Tune(filename.parse().unwrap()));
        }
        Err("Unknown command.")
    }
}

pub fn get_option_defaults() -> HashMap<&'static str, UCIOption> {
    let mut opts = HashMap::new();
    opts.insert("Hash", UCIOption::spin(16, 1, 128 * 1024));
    opts.insert("Threads", UCIOption::spin(1, 1, 512));
    opts
}

fn print_options() {
    // printing scheme from Rustfish
    let opts = get_option_defaults();
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
