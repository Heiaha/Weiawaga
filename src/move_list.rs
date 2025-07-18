use super::bitboard::*;
use super::board::*;
use super::moov::*;
use super::square::*;

use arrayvec::ArrayVec;

pub const MAX_MOVES: usize = 252;

#[derive(Default)]
pub struct MoveList(ArrayVec<Move, MAX_MOVES>);

impl MoveList {
    pub fn new() -> Self {
        Self(ArrayVec::new())
    }

    pub fn from<const QUIESCENCE: bool>(board: &Board) -> Self {
        let mut moves = Self::new();
        board.generate_legal_moves::<QUIESCENCE>(&mut moves);
        moves
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push(&mut self, m: Move) {
        self.0.push(m);
    }

    pub fn get(&self, i: usize) -> Option<&Move> {
        self.0.get(i)
    }

    pub fn contains(&self, m: Move) -> bool {
        self.0.iter().any(|entry| entry == &m)
    }

    pub fn make_q(&mut self, from_sq: SQ, to: Bitboard) {
        self.0.extend(
            to.into_iter()
                .map(|to_sq| Move::new(from_sq, to_sq, MoveFlags::Quiet)),
        );
    }

    pub fn make_c(&mut self, from_sq: SQ, to: Bitboard) {
        self.0.extend(
            to.into_iter()
                .map(|to_sq| Move::new(from_sq, to_sq, MoveFlags::Capture)),
        );
    }

    pub fn make_dp(&mut self, from_sq: SQ, to: Bitboard) {
        self.0.extend(
            to.into_iter()
                .map(|to_sq| Move::new(from_sq, to_sq, MoveFlags::DoublePush)),
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
            .map(move |flag| Move::new(from_sq, to_sq, flag))
        }));
    }

    pub fn swap(&mut self, i: usize, j: usize) {
        self.0.swap(i, j);
    }
}

impl<'a> IntoIterator for &'a MoveList {
    type Item = &'a Move;
    type IntoIter = std::slice::Iter<'a, Move>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl std::ops::Index<usize> for MoveList {
    type Output = Move;

    fn index(&self, i: usize) -> &Self::Output {
        &self.0[i]
    }
}

impl std::fmt::Debug for MoveList {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|m| {
                let (from_sq, to_sq) = m.squares();
                format!("{from_sq}{to_sq}")
            }))
            .finish()
    }
}
