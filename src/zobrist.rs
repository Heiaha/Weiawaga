use super::piece::*;
use super::square::*;
use super::types::*;

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

#[derive(Clone)]
pub struct Hasher {
    hash: u64,
    material_hash: u64,
    zobrist_table: PieceMap<SQMap<u64>>,
    zobrist_ep: FileMap<u64>,
    zobrist_color: u64,
}

impl Hasher {
    pub fn new() -> Self {
        let mut rng = StdRng::seed_from_u64(1070372);

        let mut zobrist_table = PieceMap::new([SQMap::new([0; SQ::N_SQUARES]); Piece::N_PIECES]);
        let mut zobrist_ep = FileMap::new([0; File::N_FILES]);

        let zobrist_color = rng.next_u64();

        zobrist_table
            .iter_mut()
            .flatten()
            .for_each(|hash| *hash = rng.next_u64());

        zobrist_ep
            .iter_mut()
            .for_each(|hash| *hash = rng.next_u64());

        Self {
            hash: 0,
            material_hash: 0,
            zobrist_table,
            zobrist_ep,
            zobrist_color,
        }
    }

    pub fn move_piece(&mut self, pc: Piece, from_sq: SQ, to_sq: SQ) {
        let update = self.zobrist_table[pc][from_sq] ^ self.zobrist_table[pc][to_sq];
        self.hash ^= update;
        self.material_hash ^= update;
    }

    pub fn update_piece(&mut self, pc: Piece, sq: SQ) {
        let update = self.zobrist_table[pc][sq];
        self.hash ^= update;
        self.material_hash ^= update;
    }

    pub fn update_ep(&mut self, file: File) {
        self.hash ^= self.zobrist_ep[file];
    }

    pub fn update_color(&mut self) {
        self.hash ^= self.zobrist_color;
    }

    pub fn clear(&mut self) {
        self.hash = 0;
        self.material_hash = 0;
    }

    pub fn hash(&self) -> u64 {
        self.hash
    }

    pub fn material_hash(&self) -> u64 {
        self.material_hash
    }
}
