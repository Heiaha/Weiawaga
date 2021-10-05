use super::bitboard::*;
use super::file::*;
use super::piece::*;
use super::square::*;
use rand::Rng;

static mut ZOBRIST_TABLE: [[BitBoard; N_SQUARES]; N_PIECES] =
    [[BitBoard::ZERO; N_SQUARES]; N_PIECES];
static mut ZOBRIST_EP: [BitBoard; 8] = [BitBoard::ZERO; 8];
static mut ZOBRIST_COLOR: BitBoard = BitBoard::ZERO;

#[inline(always)]
pub fn zobrist_table(pc: Piece, sq: SQ) -> BitBoard {
    unsafe { ZOBRIST_TABLE[pc as usize][sq as usize] }
}

#[inline(always)]
pub fn zobrist_ep(file: File) -> BitBoard {
    unsafe { ZOBRIST_EP[file as usize] }
}

#[inline(always)]
pub fn zobrist_color() -> BitBoard {
    unsafe { ZOBRIST_COLOR }
}

//////////////////////////////////////////////
// Inits
//////////////////////////////////////////////

fn init_zobrist_table(zobrist_table: &mut [[BitBoard; N_SQUARES]; N_PIECES]) {
    let mut rng = rand::thread_rng();
    for p in 0..N_PIECES {
        for s in 0..N_SQUARES {
            zobrist_table[p][s] = B!(rng.gen::<u64>());
        }
    }
}

fn init_zobrist_ep(zobrist_ep: &mut [BitBoard; 8]) {
    let mut rng = rand::thread_rng();
    for f in 0..8 {
        zobrist_ep[f] = B!(rng.gen::<u64>());
    }
}

fn init_zobrist_color() -> BitBoard {
    let mut rng = rand::thread_rng();
    B!(rng.gen::<u64>())
}

pub fn init_zobrist() {
    unsafe {
        init_zobrist_table(&mut ZOBRIST_TABLE);
        init_zobrist_ep(&mut ZOBRIST_EP);
        ZOBRIST_COLOR = init_zobrist_color();
    }
}
