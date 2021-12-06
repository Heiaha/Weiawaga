use super::bitboard::*;
use super::moov::*;
use super::piece::*;
use super::square::*;

#[derive(Clone, Copy, Debug)]
pub struct UndoInfo {
    entry: BitBoard,
    captured: Piece,
    epsq: SQ,
    moove: MoveInt,
    material_hash: BitBoard,
    half_move_counter: u16,
    plies_from_null: u16,
}

impl UndoInfo {
    pub const fn empty() -> Self {
        Self { entry: BitBoard::ZERO,
                   captured: Piece::None,
                   epsq: SQ::None,
                   moove: 0,
                   material_hash: BitBoard::ZERO,
                   half_move_counter: 0,
                   plies_from_null: 0 }
    }

    pub fn new(entry: BitBoard, moove: Move, half_move_counter: u16, plies_from_null: u16, captured: Piece, epsq: SQ, material_hash: BitBoard) -> Self {
        Self { entry,
                   moove: moove.moove(),
                   half_move_counter,
                   plies_from_null,
                   captured,
                   epsq,
                   material_hash }
    }

    #[inline(always)]
    pub fn entry(&self) -> BitBoard {
        self.entry
    }

    #[inline(always)]
    pub fn moove(&self) -> Move {
        Move::from(self.moove)
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
    pub fn material_hash(&self) -> BitBoard {
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
    pub fn set_entry(&mut self, entry: BitBoard) {
        self.entry = entry;
    }

    #[inline(always)]
    pub fn set_half_move_counter(&mut self, half_move_counter: u16) {
        self.half_move_counter = half_move_counter;
    }

    #[inline(always)]
    pub fn set_material_hash(&mut self, material_hash: Key) {
        self.material_hash = material_hash;
    }
}

impl Default for UndoInfo {
    fn default() -> Self {
        Self { entry: BitBoard::ZERO,
            captured: Piece::None,
            epsq: SQ::None,
            moove: 0,
            material_hash: BitBoard::ZERO,
            half_move_counter: 0,
            plies_from_null: 0 }
    }
}