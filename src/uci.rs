use super::board::Board;
use super::search_master::*;
use super::timer::*;
use regex::Regex;
use std::io::BufRead;
use std::str::FromStr;
use std::sync::LazyLock;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;
use std::{io, thread};
// Asymptote inspired a lot of this nice uci implementation.

#[allow(clippy::upper_case_acronyms)]
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
            match line.parse::<UCICommand>() {
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
                    eprintln!("{e}");
                }
            }
        }
    }
}

pub enum EngineOption {
    Hash(usize),
    Threads(u16),
    MoveOverhead(Duration),
    Ponder(bool),
    ShowWDL(bool),
    MultiPV(usize),
    ClearHash,
}

impl EngineOption {
    // Constants for Hash
    pub const HASH_MIN: usize = 1;
    pub const HASH_MAX: usize = 1048576;
    pub const HASH_DEFAULT: usize = 16;

    // Constants for Threads
    pub const THREADS_MIN: u16 = 1;
    pub const THREADS_MAX: u16 = 512;
    pub const THREADS_DEFAULT: u16 = 1;

    // Constants for MoveOverhead
    pub const MOVE_OVERHEAD_MIN: Duration = Duration::from_millis(0);
    pub const MOVE_OVERHEAD_MAX: Duration = Duration::from_millis(5000);
    pub const MOVE_OVERHEAD_DEFAULT: Duration = Duration::from_millis(10);

    // Constants for Ponder
    pub const PONDER_DEFAULT: bool = false;

    // Constants for UCI_ShowWDL
    pub const SHOW_WDL_DEFAULT: bool = false;

    // Constants for MultiPV
    pub const MULTIPV_MIN: usize = 1;
    pub const MULTIPV_MAX: usize = 255;
    pub const MULTIPV_DEFAULT: usize = 1;

    fn parse_value<T: FromStr>(value: Option<String>) -> Result<T, &'static str> {
        value
            .ok_or("No option value specified.")?
            .trim()
            .parse()
            .map_err(|_| "Unable to parse option value.")
    }

    fn parse_bool(value: Option<String>) -> Result<bool, &'static str> {
        match value
            .ok_or("No option value specified.")?
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "true" | "on" | "1" => Ok(true),
            "false" | "off" | "0" => Ok(false),
            _ => Err("Unrecognized boolean value."),
        }
    }
}

impl FromStr for EngineOption {
    type Err = &'static str;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        static OPTION_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"(?x)^
                setoption\s+
                name\s+(?P<name>.*?)
                (?:\s+value\s+(?P<value>.+))?
            $",
            )
            .expect("Failed to compile option regex.")
        });

        let re_captures = OPTION_RE.captures(line).ok_or("Unable to parse option.")?;

        let name = re_captures
            .name("name")
            .map(|m| m.as_str().to_string())
            .ok_or("Invalid name in option.")?;

        let value = re_captures.name("value").map(|m| m.as_str().to_string());

        Ok(match name.as_str() {
            "Hash" => Self::Hash(Self::parse_value(value)?),
            "Threads" => Self::Threads(Self::parse_value(value)?),
            "Move Overhead" => Self::MoveOverhead(Duration::from_millis(Self::parse_value(value)?)),
            "Ponder" => Self::Ponder(Self::parse_bool(value)?),
            "UCI_ShowWDL" => Self::ShowWDL(Self::parse_bool(value)?),
            "MultiPV" => Self::MultiPV(Self::parse_value(value)?),
            "Clear Hash" => Self::ClearHash,
            _ => return Err("Unable to parse option."),
        })
    }
}

#[allow(clippy::upper_case_acronyms)]
pub enum UCICommand {
    UCINewGame,
    UCI,
    IsReady,
    Position(Box<Board>),
    Go {
        time_control: TimeControl,
        ponder: bool,
    },
    PonderHit,
    Quit,
    Stop,
    Perft(i8),
    Option(EngineOption),
    Eval,
    Fen,
}

impl FromStr for UCICommand {
    type Err = &'static str;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
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
        let time_control = line.parse()?;
        Ok(Self::Go {
            time_control,
            ponder,
        })
    }

    fn parse_position(line: &str) -> Result<Self, &'static str> {
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

        let re_captures = POSITION_RE
            .captures(line)
            .ok_or("Invalid position format.")?;

        let fen = re_captures
            .name("startpos")
            .is_none()
            .then(|| {
                re_captures
                    .name("fen")
                    .map(|m| m.as_str())
                    .ok_or("Missing starting position.")
            })
            .transpose()?;

        let moves = re_captures
            .name("moves")
            .map(|m| m.as_str().split_whitespace().collect::<Vec<&str>>())
            .unwrap_or_default();

        let mut board = match fen {
            Some(fen) => fen.parse()?,
            None => Board::new(),
        };

        for m in moves {
            board.push_str(m)?;
        }

        Ok(Self::Position(Box::new(board)))
    }

    fn parse_option(line: &str) -> Result<Self, &'static str> {
        let engine_option = line.parse()?;
        Ok(Self::Option(engine_option))
    }

    fn parse_perft(line: &str) -> Result<Self, &'static str> {
        static PERFT_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"(?x)^
                perft\s+
                (?P<depth>.*?)
            $",
            )
            .expect("Failed to compile perft regex.")
        });

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
