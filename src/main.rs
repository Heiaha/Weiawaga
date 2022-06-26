#![allow(dead_code)]
#![allow(non_snake_case)] // Allow so we don't get a warning about the uppercase name.

use crate::search::search::init_search;
use crate::types::bitboard::init_bb;
use crate::types::magics::init_magics;
use crate::uci::uci::UCICommand;

#[macro_use]
mod types;
mod evaluation;
mod perft;
mod search;
mod uci;

fn main() {
    init_magics();
    init_bb();
    init_search();

    UCICommand::run();
}
