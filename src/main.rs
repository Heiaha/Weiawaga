#[macro_use]
mod bitboard;
mod attacks;
mod board;
mod magics;
mod moov;
mod move_list;
mod move_sorting;
mod nnue;
mod nnue_weights;
mod perft;
mod piece;
mod search;
mod search_master;
mod square;
mod timer;
mod traits;
mod tt;
mod types;
mod uci;
mod zobrist;

fn main() {
    let uci = uci::UCI::new();
    uci.run();
}
