use super::bitboard::*;
use super::moov::*;
use super::piece::*;
use super::square::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct UndoInfo {
    entry: Bitboard,
    captured: Piece,
    epsq: SQ,
    moov: MoveInt,
    material_hash: Bitboard,
    half_move_counter: u16,
    plies_from_null: u16,
}

impl UndoInfo {
    pub fn new(
        entry: Bitboard,
        m: Move,
        half_move_counter: u16,
        plies_from_null: u16,
        captured: Piece,
        epsq: SQ,
        material_hash: Bitboard,
    ) -> Self {
        Self {
            entry,
            moov: m.into(),
            half_move_counter,
            plies_from_null,
            captured,
            epsq,
            material_hash,
        }
    }

    #[inline(always)]
    pub fn entry(&self) -> Bitboard {
        self.entry
    }

    #[inline(always)]
    pub fn moov(&self) -> Move {
        Move::from(self.moov)
    }

    #[inline(always)]
    pub fn captured(&self) -> Piece {
        self.captured
    }

    #[inline(always)]
    pub fn epsq(&self) -> SQ {
        self.epsq
    }

    #[inline(always)]
    pub fn half_move_counter(&self) -> u16 {
        self.half_move_counter
    }

    #[inline(always)]
    pub fn plies_from_null(&self) -> u16 {
        self.plies_from_null
    }

    #[inline(always)]
    pub fn material_hash(&self) -> Bitboard {
        self.material_hash
    }

    #[inline(always)]
    pub fn set_captured(&mut self, pc: Piece) {
        self.captured = pc;
    }

    #[inline(always)]
    pub fn set_epsq(&mut self, sq: SQ) {
        self.epsq = sq;
    }

    #[inline(always)]
    pub fn set_entry(&mut self, entry: Bitboard) {
        self.entry = entry;
    }

    #[inline(always)]
    pub fn set_half_move_counter(&mut self, half_move_counter: u16) {
        self.half_move_counter = half_move_counter;
    }

    #[inline(always)]
    pub fn set_material_hash(&mut self, material_hash: Hash) {
        self.material_hash = material_hash;
    }
}
