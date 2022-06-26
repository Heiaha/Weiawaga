use rand::Rng;

use super::bitboard::*;
use super::file::*;
use super::piece::*;
use super::square::*;

#[derive(Clone)]
pub struct Hasher {
    hash: Hash,
    material_hash: Hash,
    zobrist_table: [Hash; SQ::N_SQUARES * Piece::N_PIECES],
    zobrist_ep: [Hash; File::N_FILES],
    zobrist_color: Hash,
}

impl Hasher {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();

        let mut zobrist_table = [Hash::ZERO; SQ::N_SQUARES * Piece::N_PIECES];
        let mut zobrist_ep = [Hash::ZERO; File::N_FILES];
        let zobrist_color = B!(rng.gen::<u64>());

        for j in 0..(SQ::N_SQUARES * Piece::N_PIECES) {
            zobrist_table[j] = B!(rng.gen::<u64>());
        }

        for j in 0..File::N_FILES {
            zobrist_ep[j] = B!(rng.gen::<u64>());
        }

        Self {
            hash: Hash::ZERO,
            material_hash: Hash::ZERO,
            zobrist_table,
            zobrist_ep,
            zobrist_color,
        }
    }

    #[inline(always)]
    pub fn move_piece(&mut self, pc: Piece, from_sq: SQ, to_sq: SQ) {
        let pc_index = pc.index() * SQ::N_SQUARES;
        let update = self.zobrist_table[pc_index + from_sq.index()]
            ^ self.zobrist_table[pc_index + to_sq.index()];
        self.hash ^= update;
        self.material_hash ^= update;
    }

    #[inline(always)]
    pub fn update_piece(&mut self, pc: Piece, sq: SQ) {
        let update = self.zobrist_table[pc.index() * SQ::N_SQUARES + sq.index()];
        self.hash ^= update;
        self.material_hash ^= update;
    }

    #[inline(always)]
    pub fn update_ep(&mut self, file: File) {
        self.hash ^= self.zobrist_ep[file.index()];
    }

    #[inline(always)]
    pub fn update_color(&mut self) {
        self.hash ^= self.zobrist_color;
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.hash = Hash::ZERO;
        self.material_hash = Hash::ZERO;
    }

    #[inline(always)]
    pub fn hash(&self) -> Hash {
        self.hash
    }

    #[inline(always)]
    pub fn material_hash(&self) -> Hash {
        self.material_hash
    }
}

// static mut ZOBRIST_TABLE: [[Bitboard; SQ::N_SQUARES]; Piece::N_PIECES] =
//     [[Bitboard::ZERO; SQ::N_SQUARES]; Piece::N_PIECES];
// static mut ZOBRIST_EP: [Bitboard; 8] = [Bitboard::ZERO; 8];
// static mut ZOBRIST_COLOR: Bitboard = Bitboard::ZERO;
//
// #[inline(always)]
// pub fn zobrist_table(pc: Piece, sq: SQ) -> Bitboard {
//     unsafe { ZOBRIST_TABLE[pc.index()][sq.index()] }
// }
//
// #[inline(always)]
// pub fn zobrist_ep(file: File) -> Bitboard {
//     unsafe { ZOBRIST_EP[file.index()] }
// }
//
// #[inline(always)]
// pub fn zobrist_color() -> Bitboard {
//     unsafe { ZOBRIST_COLOR }
// }
//
// //////////////////////////////////////////////
// // Inits
// //////////////////////////////////////////////
//
// fn init_zobrist_table(zobrist_table: &mut [[Bitboard; SQ::N_SQUARES]; Piece::N_PIECES]) {
//     let mut rng = rand::thread_rng();
//     for p in 0..Piece::N_PIECES {
//         for s in 0..SQ::N_SQUARES {
//             zobrist_table[p][s] = B!(rng.gen::<u64>());
//         }
//     }
// }
//
// fn init_zobrist_ep(zobrist_ep: &mut [Bitboard; 8]) {
//     let mut rng = rand::thread_rng();
//     for f in 0..File::N_FILES {
//         zobrist_ep[f] = B!(rng.gen::<u64>());
//     }
// }
//
// fn init_zobrist_color() -> Bitboard {
//     let mut rng = rand::thread_rng();
//     B!(rng.gen::<u64>())
// }
//
// pub fn init_zobrist() {
//     unsafe {
//         init_zobrist_table(&mut ZOBRIST_TABLE);
//         init_zobrist_ep(&mut ZOBRIST_EP);
//         ZOBRIST_COLOR = init_zobrist_color();
//     }
// }
