use super::moov::Move;
use super::bitboard::BitBoard;
use super::square::SQ;
use crate::types::moov::MoveFlags;
use std::ops::Index;

#[cfg(target_pointer_width = "128")]
pub const MAX_MOVES: usize = 248;
#[cfg(target_pointer_width = "64")]
pub const MAX_MOVES: usize = 252;
#[cfg(target_pointer_width = "32")]
pub const MAX_MOVES: usize = 254;
#[cfg(any(
target_pointer_width = "16",
target_pointer_width = "8",
))]
pub const MAX_MOVES: usize = 255;

#[derive(Debug)]
pub struct MoveList {
    list: [Move; MAX_MOVES],
    idx: usize,
    len: usize,
}

impl MoveList {
    pub fn new() -> Self {
        MoveList{
            list: [Move::empty(); MAX_MOVES],
            idx: 0,
            len: 0
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn print(&self) {
        for m in self.list {
            println!("{}", m.to_string());
        }
    }
    #[inline(always)]
    pub fn push(&mut self, m: Move) {
        self.list[self.len] = m;
        self.len += 1;
    }

    pub fn make_q(&mut self, from_sq: SQ, to: BitBoard) {
        for to_sq in to {
            self.list[self.len] = Move::new(from_sq, to_sq, MoveFlags::Quiet);
            self.len += 1;
        }
    }

    pub fn make_c(&mut self, from_sq: SQ, to: BitBoard) {
        for to_sq in to {
            self.list[self.len] = Move::new(from_sq, to_sq, MoveFlags::Capture);
            self.len += 1;
        }
    }

    pub fn make_dp(&mut self, from_sq: SQ, to: BitBoard) {
        for to_sq in to {
            self.list[self.len] = Move::new(from_sq, to_sq, MoveFlags::DoublePush);
            self.len += 1;
        }
    }

    pub fn make_pr(&mut self, from_sq: SQ, to: BitBoard) {
        for to_sq in to {
            self.list[self.len] = Move::new(from_sq, to_sq, MoveFlags::PrKnight);
            self.len += 1;

            self.list[self.len] = Move::new(from_sq, to_sq, MoveFlags::PrBishop);
            self.len += 1;

            self.list[self.len] = Move::new(from_sq, to_sq, MoveFlags::PrRook);
            self.len += 1;

            self.list[self.len] = Move::new(from_sq, to_sq, MoveFlags::PrQueen);
            self.len += 1;
        }
    }

    pub fn make_pc(&mut self, from_sq: SQ, to: BitBoard) {
        for to_sq in to {
            self.list[self.len] = Move::new(from_sq, to_sq, MoveFlags::PcKnight);
            self.len += 1;

            self.list[self.len] = Move::new(from_sq, to_sq, MoveFlags::PcBishop);
            self.len += 1;

            self.list[self.len] = Move::new(from_sq, to_sq, MoveFlags::PcRook);
            self.len += 1;

            self.list[self.len] = Move::new(from_sq, to_sq, MoveFlags::PcQueen);
            self.len += 1;
        }
    }
}

impl Iterator for MoveList {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.len {
            return None
        }
        self.idx += 1;
        Some(self.list[self.idx - 1])
    }
}

impl Index<usize> for MoveList {
    type Output = Move;

    fn index(&self, i: usize) -> &Self::Output {
        &self.list[i]
    }
}

