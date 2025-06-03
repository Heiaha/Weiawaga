use super::search_master::*;
use super::timer::*;
use regex::Regex;
use std::io::BufRead;
use std::sync::LazyLock;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::{io, thread};
// Asymptote inspired a lot of this nice uci implementation.

pub struct UCI {
    _main_thread: thread::JoinHandle<()>,
    main_tx: mpsc::Sender<UCICommand>,
    stop: Arc<AtomicBool>,
    pondering: Arc<AtomicBool>,
}

impl UCI {
    pub fn new() -> Self {
        let (main_tx, main_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let pondering = Arc::new(AtomicBool::new(false));
        Self {
            main_tx,
            stop: stop.clone(),
            pondering: pondering.clone(),
            _main_thread: thread::spawn(move || SearchMaster::new(stop, pondering).run(main_rx)),
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
            match UCICommand::try_from(line.as_str()) {
                Ok(cmd) => match cmd {
                    UCICommand::Quit => return,
                    UCICommand::Stop => {
                        self.stop.store(true, Ordering::Release);
                        self.pondering.store(false, Ordering::Release);
                    }
                    UCICommand::PonderHit => self.pondering.store(false, Ordering::Release),
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
    Position {
        fen: Option<String>,
        moves: Vec<String>,
    },
    Go {
        time_control: TimeControl,
        ponder: bool,
    },
    PonderHit,
    Quit,
    Stop,
    Perft(i8),
    Option {
        name: String,
        value: String,
    },
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
            "ponderhit" => Self::PonderHit,
            "quit" => Self::Quit,
            "isready" => Self::IsReady,
            _ => {
                if line.starts_with("go") {
                    Self::parse_go(line)?
                } else if line.starts_with("position") {
                    Self::parse_position(line)?
                } else if line.starts_with("perft") {
                    Self::parse_perft(line)?
                } else if line.starts_with("setoption") {
                    Self::parse_option(line)?
                } else {
                    return Err("Unknown command.");
                }
            }
        };
        Ok(command)
    }
}

impl UCICommand {
    fn parse_go(line: &str) -> Result<Self, &'static str> {
        let ponder = line.contains("ponder");
        let time_control = TimeControl::try_from(line)?;
        Ok(Self::Go {
            time_control,
            ponder,
        })
    }

    fn parse_position(line: &str) -> Result<Self, &'static str> {
        let re_captures = POSITION_RE
            .captures(line)
            .ok_or("Invalid position format.")?;

        let fen = re_captures
            .name("startpos")
            .is_none()
            .then(|| {
                re_captures
                    .name("fen")
                    .map(|m| m.as_str().to_string())
                    .ok_or("Missing starting position.")
            })
            .transpose()?;

        let moves = re_captures
            .name("moves")
            .map(|m| {
                m.as_str()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        Ok(Self::Position { fen, moves })
    }

    fn parse_option(line: &str) -> Result<Self, &'static str> {
        let caps = OPTION_RE
            .captures(line)
            .ok_or("Option must include a 'name' and 'value' part.")?;

        let name = caps
            .name("name")
            .map(|m| m.as_str().to_string())
            .ok_or("Invalid name in option.")?;

        let value = caps
            .name("value")
            .map(|m| m.as_str().to_string())
            .ok_or("Invalid value in option.")?;

        Ok(Self::Option { name, value })
    }

    fn parse_perft(line: &str) -> Result<Self, &'static str> {
        let re_captures = PERFT_RE.captures(line).ok_or("Invalid perft format.")?;

        re_captures
            .name("depth")
            .ok_or("Invalid perft format.")?
            .as_str()
            .parse::<i8>()
            .map_err(|_| "Invalid depth.")
            .map(Self::Perft)
    }
}

static POSITION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)^
                position\s+
                (?:(?P<startpos>startpos)|fen\s+(?P<fen>.+?))
                (\s+moves\s+(?P<moves>(?:.+?)+))?
            $",
    )
    .expect("Failed to compile position regex.")
});

static OPTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)^
                setoption\s+
                name\s+(?P<name>.*?)\s+
                value\s+(?P<value>.+)
            $",
    )
    .expect("Failed to compile option regex.")
});

static PERFT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)^
                perft\s+
                (?P<depth>.*?)
            $",
    )
    .expect("Failed to compile perft regex.")
});
