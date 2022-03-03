use super::bitboard::*;
use super::file::*;
use super::piece::*;
use super::square::*;
use rand::Rng;

static mut ZOBRIST_TABLE: [[BitBoard; SQ::N_SQUARES]; Piece::N_PIECES] =
    [[BitBoard::ZERO; SQ::N_SQUARES]; Piece::N_PIECES];
static mut ZOBRIST_EP: [BitBoard; 8] = [BitBoard::ZERO; 8];
static mut ZOBRIST_COLOR: BitBoard = BitBoard::ZERO;

#[inline(always)]
pub fn zobrist_table(pc: Piece, sq: SQ) -> BitBoard {
    unsafe { ZOBRIST_TABLE[pc.index()][sq.index()] }
}

#[inline(always)]
pub fn zobrist_ep(file: File) -> BitBoard {
    unsafe { ZOBRIST_EP[file.index()] }
}

#[inline(always)]
pub fn zobrist_color() -> BitBoard {
    unsafe { ZOBRIST_COLOR }
}

//////////////////////////////////////////////
// Inits
//////////////////////////////////////////////

fn init_zobrist_table(zobrist_table: &mut [[BitBoard; SQ::N_SQUARES]; Piece::N_PIECES]) {
    let mut rng = rand::thread_rng();
    for p in 0..Piece::N_PIECES {
        for s in 0..SQ::N_SQUARES {
            zobrist_table[p][s] = B!(rng.gen::<u64>());
        }
    }
}

fn init_zobrist_ep(zobrist_ep: &mut [BitBoard; 8]) {
    let mut rng = rand::thread_rng();
    for f in 0..File::N_FILES {
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
