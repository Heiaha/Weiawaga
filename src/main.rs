#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(non_snake_case)] // Allow so we don't get a warning about the uppercase name.

#[macro_use]
mod types;
mod evaluation;
mod perft;
mod search;
mod texel;
mod uci;

use crate::evaluation::e_constants::init_eval;
use crate::search::search::init_search;
use crate::types::bitboard::init_bb;
use crate::types::magics::init_magics;
use crate::types::zobrist::init_zobrist;
use crate::uci::uci::UCICommand;

fn main() {
    init_magics();
    init_zobrist();
    init_eval();
    init_bb();
    init_search();

    UCICommand::run();
}
