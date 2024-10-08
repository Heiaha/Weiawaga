use super::file::*;
use super::piece::*;
use super::square::*;
use super::types::*;

struct Rng(Hash);

impl Rng {
    fn gen(&mut self) -> Hash {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(2685821657736338717)
    }
}

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
        let mut rng = Rng(1070372);

        let mut zobrist_table = [0; SQ::N_SQUARES * Piece::N_PIECES];
        let mut zobrist_ep = [0; File::N_FILES];
        let zobrist_color = rng.gen();

        zobrist_table.iter_mut().for_each(|hash| *hash = rng.gen());

        zobrist_ep.iter_mut().for_each(|hash| *hash = rng.gen());

        Self {
            hash: 0,
            material_hash: 0,
            zobrist_table,
            zobrist_ep,
            zobrist_color,
        }
    }

    pub fn move_piece(&mut self, pc: Piece, from_sq: SQ, to_sq: SQ) {
        let pc_index = pc.index() * SQ::N_SQUARES;
        let update = self.zobrist_table[pc_index + from_sq.index()]
            ^ self.zobrist_table[pc_index + to_sq.index()];
        self.hash ^= update;
        self.material_hash ^= update;
    }

    pub fn update_piece(&mut self, pc: Piece, sq: SQ) {
        let update = self.zobrist_table[pc.index() * SQ::N_SQUARES + sq.index()];
        self.hash ^= update;
        self.material_hash ^= update;
    }

    pub fn update_ep(&mut self, file: File) {
        self.hash ^= self.zobrist_ep[file.index()];
    }

    pub fn update_color(&mut self) {
        self.hash ^= self.zobrist_color;
    }

    pub fn clear(&mut self) {
        self.hash = 0;
        self.material_hash = 0;
    }

    pub fn hash(&self) -> Hash {
        self.hash
    }

    pub fn material_hash(&self) -> Hash {
        self.material_hash
    }
}
