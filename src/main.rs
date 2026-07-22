#[macro_use]
mod bitboard;
mod attacks;
mod board;
mod castling;
#[cfg(feature = "datagen")]
mod datagen;
mod magics;
mod moov;
mod move_list;
mod move_sorting;
mod nnue;
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
    #[cfg(feature = "datagen")]
    if std::env::args().nth(1).as_deref() == Some("datagen") {
        datagen::run(std::env::args().skip(2).collect());
        return;
    }

    let uci = uci::UCI::new();
    uci.run();
}
