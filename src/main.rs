#![feature(step_trait)]
#![feature(destructuring_assignment)]

#[macro_use]
mod types;
mod evaluation;
mod perft;
mod search;
mod uci;

use crate::evaluation::e_constants::init_eval;
use crate::search::move_scorer::init_move_orderer;
use crate::search::search::{init_search, Search};
use crate::types::attacks::init_attacks;
use crate::types::bitboard::{init_bb, BitBoard};
use crate::types::magics::init_magics;
use crate::types::zobrist::init_zobrist;
use crate::uci::uci::UCICommand;

use crate::types::board::Board;
use crate::types::move_list::MoveList;
use crate::search::timer::{Timer, TimeControl};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use crate::types::square::SQ;
use crate::types::color::Color;
use crate::evaluation::eval::{pawn_score, eval};

fn main() {
    init_magics();
    init_attacks();
    init_zobrist();
    init_eval();
    init_bb();
    init_move_orderer();
    init_search();

    UCICommand::run();
}
