#![allow(static_mut_refs)]
#![allow(dead_code)]
#![allow(non_snake_case)] // Allow so we don't get a warning about the uppercase name.

use crate::bitboard::*;
use crate::magics::*;
use crate::search::*;
use crate::uci::*;

#[macro_use]
mod bitboard;
mod attacks;
mod board;
mod magics;
mod moov;
mod move_list;
mod move_sorter;
mod nnue;
mod nnue_weights;
mod perft;
mod piece;
mod search;
mod search_master;
mod square;
mod timer;
mod tt;
mod types;
mod uci;
mod zobrist;

fn main() {
    init_magics();
    init_bb();
    init_search();

    let uci = UCI::new();
    uci.run();
}
