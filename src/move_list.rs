use std::fmt;
use std::ops::Index;

use super::bitboard::*;
use super::board::*;
use super::moov::*;
use super::square::*;
use super::types::*;

// Cache size consideration idea originally found in Pleco

// Make sure the move lists are aligned into lengths such that the memory is
// in an integer number of cache chunks. The is for a 16 bit Move.
// https://www.youtube.com/watch?v=WDIkqP4JbkE

#[cfg(target_pointer_width = "128")]
const MAX_MOVES: usize = 248;
#[cfg(target_pointer_width = "64")]
pub const MAX_MOVES: usize = 252;
#[cfg(target_pointer_width = "32")]
const MAX_MOVES: usize = 254;
#[cfg(any(target_pointer_width = "16", target_pointer_width = "8",))]
const MAX_MOVES: usize = 255;

pub struct MoveList {
    moves: [Move; MAX_MOVES],
    pub scores: [Value; MAX_MOVES],
    idx: usize,
    len: usize,
}

impl MoveList {
    pub fn new() -> Self {
        Self {
            moves: [Move::NULL; MAX_MOVES],
            scores: [0; MAX_MOVES],
            idx: 0,
            len: 0,
        }
    }

    pub fn from(board: &Board) -> Self {
        let mut moves = Self::new();
        board.generate_legal_moves(&mut moves);
        moves
    }

    pub fn from_q(board: &Board) -> Self {
        let mut moves = Self::new();
        board.generate_legal_q_moves(&mut moves);
        moves
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn contains(&self, m: Move) -> bool {
        self.moves[..self.len].contains(&m)
    }

    #[inline(always)]
    pub fn push(&mut self, m: Move) {
        self.moves[self.len] = m;
        self.len += 1;
    }

    pub fn make_q(&mut self, from_sq: SQ, to: Bitboard) {
        for to_sq in to {
            self.moves[self.len] = Move::new(from_sq, to_sq, MoveFlags::Quiet);
            self.len += 1;
        }
    }

    pub fn make_c(&mut self, from_sq: SQ, to: Bitboard) {
        for to_sq in to {
            self.moves[self.len] = Move::new(from_sq, to_sq, MoveFlags::Capture);
            self.len += 1;
        }
    }

    pub fn make_dp(&mut self, from_sq: SQ, to: Bitboard) {
        for to_sq in to {
            self.moves[self.len] = Move::new(from_sq, to_sq, MoveFlags::DoublePush);
            self.len += 1;
        }
    }

    pub fn make_pc(&mut self, from_sq: SQ, to: Bitboard) {
        for to_sq in to {
            self.moves[self.len] = Move::new(from_sq, to_sq, MoveFlags::PcKnight);
            self.len += 1;

            self.moves[self.len] = Move::new(from_sq, to_sq, MoveFlags::PcBishop);
            self.len += 1;

            self.moves[self.len] = Move::new(from_sq, to_sq, MoveFlags::PcRook);
            self.len += 1;

            self.moves[self.len] = Move::new(from_sq, to_sq, MoveFlags::PcQueen);
            self.len += 1;
        }
    }

    pub fn next_best(&mut self) -> Option<Move> {
        if self.idx == self.len {
            return None;
        }

        let mut max_score = Value::MIN;
        let mut max_idx = self.idx;

        for i in self.idx..self.len() {
            if self.scores[i] > max_score {
                max_idx = i;
                max_score = self.scores[i];
            }
        }
        self.moves.swap(self.idx, max_idx);
        self.scores.swap(self.idx, max_idx);
        self.idx += 1;
        Some(self.moves[self.idx - 1])
    }
}

impl Iterator for MoveList {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.len {
            self.idx = 0;
            return None;
        }
        self.idx += 1;
        Some(self.moves[self.idx - 1])
    }
}

impl Index<usize> for MoveList {
    type Output = Move;

    fn index(&self, i: usize) -> &Self::Output {
        &self.moves[i]
    }
}

impl fmt::Debug for MoveList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut result = String::from('[');
        for i in 0..self.len {
            result.push_str(format!("{}, ", self.moves[i].to_string()).as_ref());
        }
        result.push(']');
        write!(f, "{}", result)
    }
}
