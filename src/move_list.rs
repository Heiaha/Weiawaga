use super::bitboard::*;
use super::board::*;
use super::moov::*;
use super::square::*;
use super::types::*;

use arrayvec::ArrayVec;

pub const MAX_MOVES: usize = 252;

pub struct MoveListEntry {
    pub m: Move,
    pub score: Value,
}

pub struct MoveList(ArrayVec<MoveListEntry, MAX_MOVES>);

impl MoveListEntry {
    pub fn new(m: Move) -> Self {
        MoveListEntry { m, score: 0 }
    }
}

impl MoveList {
    pub fn new() -> Self {
        Self(ArrayVec::new())
    }

    pub fn from(board: &Board) -> Self {
        let mut moves = Self::new();
        board.generate_legal_moves::<true>(&mut moves);
        moves
    }

    pub fn from_q(board: &Board) -> Self {
        let mut moves = Self::new();
        board.generate_legal_moves::<false>(&mut moves);
        moves
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push(&mut self, m: Move) {
        self.0.push(MoveListEntry::new(m));
    }

    pub fn contains(&self, m: Move) -> bool {
        self.0.iter().any(|entry| entry.m == m)
    }

    pub fn make_q(&mut self, from_sq: SQ, to: Bitboard) {
        for to_sq in to {
            self.0.push(MoveListEntry::new(Move::new(
                from_sq,
                to_sq,
                MoveFlags::Quiet,
            )));
        }
    }

    pub fn make_c(&mut self, from_sq: SQ, to: Bitboard) {
        for to_sq in to {
            self.0.push(MoveListEntry::new(Move::new(
                from_sq,
                to_sq,
                MoveFlags::Capture,
            )));
        }
    }

    pub fn make_dp(&mut self, from_sq: SQ, to: Bitboard) {
        for to_sq in to {
            self.0.push(MoveListEntry::new(Move::new(
                from_sq,
                to_sq,
                MoveFlags::DoublePush,
            )));
        }
    }

    pub fn make_pc(&mut self, from_sq: SQ, to: Bitboard) {
        for to_sq in to {
            for flag in [
                MoveFlags::PcQueen,
                MoveFlags::PcKnight,
                MoveFlags::PcRook,
                MoveFlags::PcBishop,
            ] {
                self.0
                    .push(MoveListEntry::new(Move::new(from_sq, to_sq, flag)));
            }
        }
    }

    pub fn iter_moves(&self) -> impl Iterator<Item = Move> + '_ {
        self.0.iter().map(|entry| entry.m)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut MoveListEntry> {
        self.0.iter_mut()
    }

    pub fn next_best(&mut self, idx: usize) -> Option<Move> {
        if idx == self.len() {
            return None;
        }

        let mut max_score = Value::MIN;
        let mut max_idx = idx;

        for i in idx..self.len() {
            if self.0[i].score > max_score {
                max_idx = i;
                max_score = self.0[i].score;
            }
        }

        self.0.swap(idx, max_idx);

        Some(self.0[idx].m)
    }
}

impl<'a> IntoIterator for &'a mut MoveList {
    type Item = &'a mut MoveListEntry;
    type IntoIter = std::slice::IterMut<'a, MoveListEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<'a> IntoIterator for &'a MoveList {
    type Item = &'a MoveListEntry;
    type IntoIter = std::slice::Iter<'a, MoveListEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl std::ops::Index<usize> for MoveList {
    type Output = MoveListEntry;

    fn index(&self, i: usize) -> &Self::Output {
        &self.0[i]
    }
}

impl std::fmt::Debug for MoveList {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut result = String::from('[');
        for i in 0..self.len() {
            let m = self.0[i].m;
            result.push_str(format!("{}, ", m).as_str());
        }
        result.push(']');
        write!(f, "{}", result)
    }
}
