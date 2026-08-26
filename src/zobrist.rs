use super::castling::*;
use super::piece::*;
use super::square::*;
use std::sync::LazyLock;

pub static ZOBRIST: LazyLock<Hasher> = LazyLock::new(Hasher::new);

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

pub struct Hasher {
    zobrist_table: PieceMap<SQMap<u64>>,
    zobrist_ep: FileMap<u64>,
    zobrist_color: ColorMap<u64>,
    zobrist_castling: CastlingMap<u64>,
}

impl Hasher {
    pub fn new() -> Self {
        let mut zobrist_table = PieceMap::<SQMap<u64>>::default();
        let mut zobrist_ep = FileMap::<u64>::default();

        let mut rng = SplitMix64(1070372);

        zobrist_table
            .iter_mut()
            .flatten()
            .for_each(|hash| *hash = rng.next_u64());

        zobrist_ep
            .iter_mut()
            .for_each(|hash| *hash = rng.next_u64());

        let zobrist_color = ColorMap::new([rng.next_u64(), rng.next_u64()]);

        let mut zobrist_castling = CastlingMap::default();
        zobrist_castling
            .iter_mut()
            .for_each(|hash| *hash = rng.next_u64());

        Self {
            zobrist_table,
            zobrist_ep,
            zobrist_color,
            zobrist_castling,
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

    pub fn castling_hash(&self, rights: CastlingRights) -> u64 {
        self.zobrist_castling[rights]
    }

    pub fn color_hash(&self, color: Color) -> u64 {
        self.zobrist_color[color]
    }
}
