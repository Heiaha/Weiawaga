#![feature(step_trait)]
#![feature(destructuring_assignment)]

#[macro_use]
mod types;
mod evaluation;
mod perft;
mod search;
mod uci;
mod texel;

use crate::evaluation::e_constants::init_eval;
use crate::search::move_sorter::init_move_orderer;
use crate::search::search::init_search;
use crate::texel::tuner::Tuner;
use crate::types::bitboard::init_bb;
use crate::types::magics::init_magics;
use crate::types::zobrist::init_zobrist;
use crate::uci::uci::UCICommand;

fn main() {
    init_magics();
    init_zobrist();
    init_eval();
    init_bb();
    init_move_orderer();
    init_search();

    UCICommand::run();
}
