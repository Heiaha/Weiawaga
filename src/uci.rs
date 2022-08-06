use std::io::BufRead;
use std::{io, sync, thread};

use super::board::*;
use super::search_master::*;
use super::timer::*;
use super::types::*;

// A lot of this nice uci implementation was inspired by Asymptote.

pub struct UCI {
    _main_thread: thread::JoinHandle<()>,
    main_tx: sync::mpsc::Sender<UCICommand>,
    stop: sync::Arc<sync::atomic::AtomicBool>,
}

impl UCI {
    pub fn new() -> Self {
        let (main_tx, main_rx) = sync::mpsc::channel();
        let stop = sync::Arc::new(sync::atomic::AtomicBool::new(false));
        Self {
            main_tx,
            stop: stop.clone(),
            _main_thread: thread::spawn(move || SearchMaster::new(stop).run(main_rx)),
        }
    }

    pub fn run(&self) {
        println!("Weiawaga v{}", env!("CARGO_PKG_VERSION"));
        println!("{}", env!("CARGO_PKG_REPOSITORY"));

        let stdin = io::stdin();
        let lock = stdin.lock();

        for line in lock
            .lines()
            .map(|line| line.expect("Unable to parse line."))
        {
            match UCICommand::try_from(line.as_ref()) {
                Ok(cmd) => match cmd {
                    UCICommand::Quit => return,
                    UCICommand::Stop => self.stop.store(true, sync::atomic::Ordering::SeqCst),
                    cmd => self
                        .main_tx
                        .send(cmd)
                        .expect("Unable to communicate with main thread."),
                },
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
        }
    }
}

pub enum UCICommand {
    UCINewGame,
    UCI,
    IsReady,
    Position(Board),
    Go(TimeControl),
    Quit,
    Stop,
    Perft(Depth),
    Option(String, String),
    Eval,
}

impl TryFrom<&str> for UCICommand {
    type Error = &'static str;

    fn try_from(line: &str) -> Result<Self, Self::Error> {
        if line.replace(" ", "") == "ucinewgame" {
            Ok(Self::UCINewGame)
        } else if line.replace(" ", "") == "stop" {
            Ok(Self::Stop)
        } else if line.replace(" ", "") == "uci" {
            Ok(Self::UCI)
        } else if line.replace(" ", "") == "eval" {
            Ok(Self::Eval)
        } else if line.replace(" ", "") == "quit" {
            Ok(Self::Quit)
        } else if line.replace(" ", "") == "isready" {
            Ok(Self::IsReady)
        } else if line.starts_with("go") {
            let time_control = TimeControl::from(line);
            Ok(Self::Go(time_control))
        } else if line.starts_with("position") {
            let mut board;
            let fen = line.trim_start_matches("position ");
            if fen.starts_with("startpos") {
                board = Board::new();
            } else {
                board = Board::try_from(fen.trim_start_matches("fen"))?;
            }
            if line.contains("moves") {
                if let Some(moves_str) = line.split_terminator("moves ").nth(1) {
                    for mov in moves_str.split_whitespace() {
                        board.push_str(mov)?;
                    }
                }
            }
            Ok(Self::Position(board))
        } else if line.starts_with("perft") {
            let depth = line
                .split_whitespace()
                .nth(1)
                .and_then(|d| d.parse().ok())
                .unwrap_or(6);
            Ok(Self::Perft(depth))
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
            Ok(Self::Option(name.parse().unwrap(), value.parse().unwrap()))
        } else {
            Err("Unknown command.")
        }
    }
}
