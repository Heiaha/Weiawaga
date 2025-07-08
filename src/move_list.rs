use super::bitboard::*;
use super::board::*;
use super::moov::*;
use super::square::*;

use arrayvec::ArrayVec;

pub const MAX_MOVES: usize = 252;

pub struct MoveListEntry {
    pub m: Move,
    pub score: i32,
}

impl MoveListEntry {
    pub fn new(m: Move) -> Self {
        MoveListEntry { m, score: 0 }
    }
}

#[derive(Default)]
pub struct MoveList(ArrayVec<MoveListEntry, MAX_MOVES>);

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
        self.0.extend(
            to.into_iter()
                .map(|to_sq| MoveListEntry::new(Move::new(from_sq, to_sq, MoveFlags::Quiet))),
        );
    }

    pub fn make_c(&mut self, from_sq: SQ, to: Bitboard) {
        self.0.extend(
            to.into_iter()
                .map(|to_sq| MoveListEntry::new(Move::new(from_sq, to_sq, MoveFlags::Capture))),
        );
    }

    pub fn make_dp(&mut self, from_sq: SQ, to: Bitboard) {
        self.0.extend(
            to.into_iter()
                .map(|to_sq| MoveListEntry::new(Move::new(from_sq, to_sq, MoveFlags::DoublePush))),
        );
    }

    pub fn make_pc(&mut self, from_sq: SQ, to: Bitboard) {
        self.0.extend(to.into_iter().flat_map(|to_sq| {
            [
                MoveFlags::PcQueen,
                MoveFlags::PcKnight,
                MoveFlags::PcRook,
                MoveFlags::PcBishop,
            ]
            .into_iter()
            .map(move |flag| MoveListEntry::new(Move::new(from_sq, to_sq, flag)))
        }));
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

        let max_idx = (idx..self.len()).max_by_key(|&i| self.0[i].score)?;

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
        f.debug_list()
            .entries(self.0.iter().map(|entry| {
                let (from_sq, to_sq) = entry.m.squares();
                format!("{}{}", from_sq, to_sq)
            }))
            .finish()
    }
}
