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
    Position(Option<String>, Vec<String>),
    Go(TimeControl),
    Quit,
    Stop,
    Perft(Depth),
    Option(String, String),
    Eval,
    Fen,
}

impl TryFrom<&str> for UCICommand {
    type Error = &'static str;

    fn try_from(line: &str) -> Result<Self, Self::Error> {
        let line = line.trim();

        let command = match line {
            "ucinewgame" => Self::UCINewGame,
            "stop" => Self::Stop,
            "uci" => Self::UCI,
            "eval" => Self::Eval,
            "fen" => Self::Fen,
            "quit" => Self::Quit,
            "isready" => Self::IsReady,
            _ => {
                if let Some(tc_info) = line.strip_prefix("go") {
                    let time_control = TimeControl::try_from(tc_info)?;
                    Self::Go(time_control)
                } else if let Some(position_str) = line.strip_prefix("position") {
                    let position_str = position_str.trim();
                    let fen = if position_str.starts_with("startpos") {
                        None
                    } else if let Some(fen_str) = position_str.strip_prefix("fen") {
                        let fen_parts: Vec<&str> = fen_str
                            .split_whitespace()
                            .take_while(|p| *p != "moves")
                            .collect();

                        if !fen_parts.is_empty() {
                            Some(fen_parts.join(" "))
                        } else {
                            None
                        }
                    } else {
                        return Err("Unable to parse position.");
                    };

                    let move_strs = position_str
                        .split_whitespace()
                        .skip_while(|p| *p != "moves")
                        .skip(1)
                        .map(String::from)
                        .collect();

                    Self::Position(fen, move_strs)
                } else if let Some(perft_depth) = line.strip_prefix("perft ") {
                    let depth = perft_depth.parse().or(Err("Unable to parse depth."))?;
                    Self::Perft(depth)
                } else if let Some(option_info) = line.strip_prefix("setoption ") {
                    let mut option_iter = option_info.split_whitespace();
                    if option_iter.next() != Some("name") {
                        return Err("Option must include a 'name' part.");
                    }
                    let name = option_iter
                        .by_ref()
                        .take_while(|word| *word != "value")
                        .collect();
                    let value = option_iter.collect();
                    Self::Option(name, value)
                } else {
                    return Err("Unknown command.");
                }
            }
        };

        Ok(command)
    }
}
