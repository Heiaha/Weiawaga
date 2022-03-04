use super::bitboard::*;
use super::file::*;
use super::piece::*;
use super::square::*;
use rand::Rng;

static mut ZOBRIST_TABLE: [[Bitboard; SQ::N_SQUARES]; Piece::N_PIECES] =
    [[Bitboard::ZERO; SQ::N_SQUARES]; Piece::N_PIECES];
static mut ZOBRIST_EP: [Bitboard; 8] = [Bitboard::ZERO; 8];
static mut ZOBRIST_COLOR: Bitboard = Bitboard::ZERO;

#[inline(always)]
pub fn zobrist_table(pc: Piece, sq: SQ) -> Bitboard {
    unsafe { ZOBRIST_TABLE[pc.index()][sq.index()] }
}

#[inline(always)]
pub fn zobrist_ep(file: File) -> Bitboard {
    unsafe { ZOBRIST_EP[file.index()] }
}

#[inline(always)]
pub fn zobrist_color() -> Bitboard {
    unsafe { ZOBRIST_COLOR }
}

//////////////////////////////////////////////
// Inits
//////////////////////////////////////////////

fn init_zobrist_table(zobrist_table: &mut [[Bitboard; SQ::N_SQUARES]; Piece::N_PIECES]) {
    let mut rng = rand::thread_rng();
    for p in 0..Piece::N_PIECES {
        for s in 0..SQ::N_SQUARES {
            zobrist_table[p][s] = B!(rng.gen::<u64>());
        }
    }
}

fn init_zobrist_ep(zobrist_ep: &mut [Bitboard; 8]) {
    let mut rng = rand::thread_rng();
    for f in 0..File::N_FILES {
        zobrist_ep[f] = B!(rng.gen::<u64>());
    }
}

fn init_zobrist_color() -> Bitboard {
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
