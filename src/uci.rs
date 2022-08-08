use std::io::BufRead;
use std::{io, sync, thread};

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
                    _ => self
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
    Board(Option<String>, Vec<String>),
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
        let line = line.trim();
        if line == "ucinewgame" {
            Ok(Self::UCINewGame)
        } else if line == "stop" {
            Ok(Self::Stop)
        } else if line == "uci" {
            Ok(Self::UCI)
        } else if line == "eval" {
            Ok(Self::Eval)
        } else if line == "quit" {
            Ok(Self::Quit)
        } else if line == "isready" {
            Ok(Self::IsReady)
        } else if line.starts_with("go") {
            let time_control = TimeControl::try_from(line)?;
            Ok(Self::Go(time_control))
        } else if line.starts_with("position") {
            let position_str = line.trim_start_matches("position ");
            let fen = if position_str.starts_with("startpos") {
                None
            } else {
                Some(String::from(position_str.trim_start_matches("fen")))
            };
            let mut move_strs = Vec::new();
            if line.contains("moves") {
                if let Some(moves_str) = line.split_terminator("moves ").nth(1) {
                    moves_str
                        .split_whitespace()
                        .map(String::from)
                        .for_each(|move_str| move_strs.push(move_str));
                }
            }
            Ok(Self::Board(fen, move_strs))
        } else if line.starts_with("perft") {
            let depth = line
                .split_whitespace()
                .nth(1)
                .ok_or("perft command requires a depth.")?
                .parse()
                .or(Err("Unable to parse depth."))?;
            Ok(Self::Perft(depth))
        } else if line.starts_with("setoption") {
            let mut words = line
                .split_terminator("setoption name ")
                .nth(1)
                .ok_or("Could not parse option.")?
                .split_whitespace()
                .into_iter();

            let name = words
                .by_ref()
                .take_while(|word| *word != "value")
                .map(String::from)
                .fold(String::new(), |a, b| format!("{} {}", a, b))
                .trim()
                .to_string();

            let value = words
                .by_ref()
                .map(String::from)
                .fold(String::new(), |a, b| format!("{} {}", a, b))
                .trim()
                .to_string();

            Ok(Self::Option(name, value))
        } else {
            Err("Unknown command.")
        }
    }
}
