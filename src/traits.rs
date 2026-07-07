use super::bitboard::*;
use super::piece::*;

pub trait Relative: Copy + Sized {
    fn vmirror(&self) -> Self;

    fn relative(&self, color: Color) -> Self {
        match color {
            Color::White => *self,
            Color::Black => self.vmirror(),
        }
    }
}

pub trait Enumerable: Copy + Sized {
    const COUNT: usize;

    fn from_repr(n: u8) -> Self {
        const {
            assert!(std::mem::size_of::<Self>() == 1);
        }
        debug_assert!((n as usize) < Self::COUNT);
        unsafe { std::mem::transmute_copy(&n) }
    }

    fn index(&self) -> usize {
        const {
            assert!(std::mem::size_of::<Self>() == 1);
        }
        unsafe { std::mem::transmute_copy::<Self, u8>(self) as usize }
    }

    fn iter() -> impl Iterator<Item = Self> {
        (0..Self::COUNT as u8).map(Self::from_repr)
    }

    fn range(start: Self, end: Self) -> impl Iterator<Item = Self> {
        (start.index() as u8..=end.index() as u8).map(Self::from_repr)
    }
}

pub trait BitboardMask: Enumerable {
    const MASKS: &'static [Bitboard];
    fn bb(self) -> Bitboard {
        Self::MASKS[self.index()]
    }
}
