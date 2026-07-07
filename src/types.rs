use super::piece::*;
use super::square::*;
use super::traits::*;
use std::ops::{Index, IndexMut};
use std::slice::{Iter, IterMut};

// Maximum distance from the root, used to size per-ply tables.
pub const MAX_PLY: usize = 256;

pub type ColorMap<T> = EnumMap<T, { Color::COUNT }>;
pub type PieceMap<T> = EnumMap<T, { Piece::COUNT }>;
pub type PieceTypeMap<T> = EnumMap<T, { PieceType::COUNT }>;
pub type SQMap<T> = EnumMap<T, { SQ::COUNT }>;
pub type FileMap<T> = EnumMap<T, { File::COUNT }>;

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
    E: Enumerable,
{
    type Output = T;

    fn index(&self, key: E) -> &Self::Output {
        let idx = key.index();
        debug_assert!(idx < N);
        &self.0[idx]
    }
}

impl<T, E, const N: usize> IndexMut<E> for EnumMap<T, N>
where
    E: Enumerable,
{
    fn index_mut(&mut self, key: E) -> &mut Self::Output {
        let idx = key.index();
        debug_assert!(idx < N);
        &mut self.0[idx]
    }
}

impl<T: Copy + Default, const N: usize> Default for EnumMap<T, N> {
    fn default() -> Self {
        Self([T::default(); N])
    }
}

pub trait Score {
    fn is_checkmate(&self) -> bool;

    const MATE: i32 = 32000;
}

impl Score for i32 {
    fn is_checkmate(&self) -> bool {
        self.abs() >= Self::MATE >> 1
    }
}
