use super::piece::*;
use super::square::*;
use super::types::*;
use std::sync::LazyLock;

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

pub static ZOBRIST: LazyLock<Hasher> = LazyLock::new(Hasher::new);

#[derive(Clone)]
pub struct Hasher {
    zobrist_table: PieceMap<SQMap<u64>>,
    zobrist_ep: FileMap<u64>,
    zobrist_color: ColorMap<u64>,
}

impl Hasher {
    pub fn new() -> Self {
        let mut zobrist_table = PieceMap::new([SQMap::new([0; SQ::N_SQUARES]); Piece::N_PIECES]);
        let mut zobrist_ep = FileMap::new([0; File::N_FILES]);

        let mut rng = StdRng::seed_from_u64(1070372);

        zobrist_table
            .iter_mut()
            .flatten()
            .for_each(|hash| *hash = rng.next_u64());

        zobrist_ep
            .iter_mut()
            .for_each(|hash| *hash = rng.next_u64());

        let zobrist_color = ColorMap::new([rng.next_u64(), rng.next_u64()]);

        Self {
            zobrist_table,
            zobrist_ep,
            zobrist_color,
        }
    }

    pub fn move_hash(&self, pc: Piece, from_sq: SQ, to_sq: SQ) -> u64 {
        self.zobrist_table[pc][from_sq] ^ self.zobrist_table[pc][to_sq]
    }

    pub fn update_hash(&self, pc: Piece, sq: SQ) -> u64 {
        self.zobrist_table[pc][sq]
    }

    pub fn ep_hash(&self, epsq: SQ) -> u64 {
        self.zobrist_ep[epsq.file()]
    }

    pub fn color_hash(&self, color: Color) -> u64 {
        self.zobrist_color[color]
    }
}
