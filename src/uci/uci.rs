use crate::perft::perft::*;
use crate::search::search::*;
use crate::search::timer::*;
use crate::search::tt::*;
use crate::types::board::*;
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{io, sync, thread};

pub enum UCICommand {
    Unknown(String),
    UCINewGame,
    UCI,
    IsReady,
    Position(Board, Vec<String>),
    Go(TimeControl),
    Quit,
    Stop,
    Perft(u8),
    Option(String, String),
}

const HASH_DEFAULT: u64 = 512;

fn thread_loop(thread: sync::mpsc::Receiver<UCICommand>, abort: Arc<AtomicBool>) {
    // global board
    let mut board = Board::new();

    // global transposition table
    let mut tt: TT = TT::new(HASH_DEFAULT);
    for cmd in thread {
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
                    board.push_str(m);
                }
            }
            UCICommand::Go(time_control) => {
                let timer = Timer::new(&board, time_control, abort.clone());
                let mut search = Search::new(timer, &mut tt);
                let (best_move, best_score) = search.go(&mut board);
                println!("info score cp {}", best_score);
                println!("bestmove {}", best_move.to_string());
                tt.clear();
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
            _ => {
                println!("Unexpected UCI Command.");
            }
        }
    }
}

impl UCICommand {
    pub fn run() {
        println!("Weiawaga v3.0 September 14, 2021");
        println!("Homepage and source code: https://github.com/Heiaha/Weiawaga");
        let stdin = io::stdin();
        let lock = stdin.lock();

        let thread_moved_abort = sync::Arc::new(sync::atomic::AtomicBool::new(false));
        let abort = sync::Arc::clone(&thread_moved_abort);
        let (main_tx, main_rx) = sync::mpsc::channel();
        let builder = thread::Builder::new()
            .name("Main thread".into())
            .stack_size(8 * 1024 * 1024);
        let thread = builder
            .spawn(move || thread_loop(main_rx, thread_moved_abort))
            .unwrap();

        for line in lock.lines() {
            let cmd = UCICommand::from(&*line.unwrap().to_owned());
            match cmd {
                UCICommand::Quit => return,
                UCICommand::Stop => {
                    abort.store(true, Ordering::SeqCst);
                }
                UCICommand::UCI => {
                    println!("id name Weiawaga");
                    println!("id author Malarksist");
                    println!("option name Hash type spin default {default}", default=HASH_DEFAULT);
                    println!("uciok");
                }
                cmd => {
                    main_tx.send(cmd).unwrap();
                }
            }
        }
    }
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
            while let Some(word) = words.next() {
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
                if let Some(moves_) = line.split_terminator("moves ").nth(1) {
                    for mov in moves_.split_whitespace() {
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
        }
        Self::Unknown(line.to_owned())
    }
}
