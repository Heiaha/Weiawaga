use super::piece::*;
use super::square::*;
use std::ops::{Index, IndexMut};
use std::slice::{Iter, IterMut};

pub type ColorMap<T> = EnumMap<T, { Color::N_COLORS }>;
pub type PieceMap<T> = EnumMap<T, { Piece::N_PIECES }>;
pub type PieceTypeMap<T> = EnumMap<T, { PieceType::N_PIECE_TYPES }>;
pub type SQMap<T> = EnumMap<T, { SQ::N_SQUARES }>;
pub type FileMap<T> = EnumMap<T, { File::N_FILES }>;
pub type RankMap<T> = EnumMap<T, { Rank::N_RANKS }>;
pub type DiagonalMap<T> = EnumMap<T, { Diagonal::N_DIAGONALS }>;

#[derive(Copy, Clone)]
pub struct EnumMap<T, const N: usize>([T; N]);

impl<T, const N: usize> EnumMap<T, N> {
    pub const fn new(data: [T; N]) -> EnumMap<T, N> {
        Self(data)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.0.iter_mut()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut EnumMap<T, N> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a EnumMap<T, N> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T, E, const N: usize> Index<E> for EnumMap<T, N>
where
    E: Into<usize>,
{
    type Output = T;

    fn index(&self, key: E) -> &Self::Output {
        let idx = key.into();
        debug_assert!(idx < N);
        &self.0[idx]
    }
}

impl<T, E, const N: usize> IndexMut<E> for EnumMap<T, N>
where
    E: Into<usize>,
{
    fn index_mut(&mut self, key: E) -> &mut Self::Output {
        let idx = key.into();
        debug_assert!(idx < N);
        &mut self.0[idx]
    }
}
