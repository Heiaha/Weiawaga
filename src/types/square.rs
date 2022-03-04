use super::bitboard::*;
use super::color::*;
use super::diagonal::*;
use super::file::*;
use super::rank::*;
use std::fmt;
use std::ops::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
#[rustfmt::skip]
pub enum SQ {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
    None
}

impl SQ {
    pub fn encode(rank: Rank, file: File) -> Self {
        Self::from(((rank as u8) << 3) + (file as u8))
    }

    #[inline(always)]
    pub fn bb(self) -> Bitboard {
        Self::SQUARES_BB[self as usize]
    }

    #[inline(always)]
    pub fn index(self) -> usize {
        self as usize
    }

    #[inline(always)]
    pub fn rank(self) -> Rank {
        Rank::from(self as u8 >> 3)
    }

    #[inline(always)]
    pub fn file(self) -> File {
        File::from(self as u8 & 7)
    }

    pub fn diagonal(self) -> Diagonal {
        let value = self as u8;
        Diagonal::from(7 + (value >> 3) - (value & 7))
    }

    pub fn antidiagonal(self) -> AntiDiagonal {
        let value = self as u8;
        AntiDiagonal::from((value >> 3) + (value & 7))
    }

    #[inline(always)]
    pub fn square_mirror(self) -> Self {
        Self::from(self as u8 ^ 0x38)
    }

    #[inline]
    pub fn forward_ranks_bb(self, color: Color) -> Bitboard {
        if color == Color::White {
            !Rank::One.bb() << (8 * self.rank().relative(color) as u8)
        } else {
            !Rank::Eight.bb() >> (8 * self.rank().relative(color) as u8)
        }
    }

    pub fn forward_files_bb(self, color: Color) -> Bitboard {
        self.file().bb() & self.forward_ranks_bb(color)
    }

    #[inline(always)]
    pub fn relative(self, color: Color) -> Self {
        if color == Color::White {
            self
        } else {
            self.square_mirror()
        }
    }

    #[inline(always)]
    pub fn iter(start: Self, end: Self) -> impl Iterator<Item = Self> {
        (start as u8..=end as u8).map(Self::from)
    }
}

impl Add<Direction> for SQ {
    type Output = Self;

    #[inline(always)]
    fn add(self, dir: Direction) -> Self {
        Self::from((self as u8).wrapping_add(dir as u8))
    }
}

impl Sub<Direction> for SQ {
    type Output = Self;

    #[inline(always)]
    fn sub(self, dir: Direction) -> Self {
        Self::from((self as u8).wrapping_sub(dir as u8))
    }
}

impl From<u8> for SQ {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}

impl Default for SQ {
    fn default() -> Self {
        Self::None
    }
}

impl fmt::Display for SQ {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Self::SQ_DISPLAY[*self as usize])
    }
}

impl TryFrom<&str> for SQ {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        return Ok(Self::from(
            Self::SQ_DISPLAY
                .iter()
                .position(|potential_sq_str| *potential_sq_str == value)
                .ok_or("Invalid square.")? as u8,
        ));
    }
}

impl SQ {
    pub const N_SQUARES: usize = 64;

    #[rustfmt::skip]
    const SQUARES_BB: [Bitboard; Self::N_SQUARES + 1] = [
        B!(1 << 0),  B!(1 << 1),  B!(1 << 2),  B!(1 << 3),  B!(1 << 4),  B!(1 << 5),  B!(1 << 6),  B!(1 << 7),
        B!(1 << 8),  B!(1 << 9),  B!(1 << 10), B!(1 << 11), B!(1 << 12), B!(1 << 13), B!(1 << 14), B!(1 << 15),
        B!(1 << 16), B!(1 << 17), B!(1 << 18), B!(1 << 19), B!(1 << 20), B!(1 << 21), B!(1 << 22), B!(1 << 23),
        B!(1 << 24), B!(1 << 25), B!(1 << 26), B!(1 << 27), B!(1 << 28), B!(1 << 29), B!(1 << 30), B!(1 << 31),
        B!(1 << 32), B!(1 << 33), B!(1 << 34), B!(1 << 35), B!(1 << 36), B!(1 << 37), B!(1 << 38), B!(1 << 39),
        B!(1 << 40), B!(1 << 41), B!(1 << 42), B!(1 << 43), B!(1 << 44), B!(1 << 45), B!(1 << 46), B!(1 << 47),
        B!(1 << 48), B!(1 << 49), B!(1 << 50), B!(1 << 51), B!(1 << 52), B!(1 << 53), B!(1 << 54), B!(1 << 55),
        B!(1 << 56), B!(1 << 57), B!(1 << 58), B!(1 << 59), B!(1 << 60), B!(1 << 61), B!(1 << 62), B!(1 << 63),
        B!(0)
    ];

    #[rustfmt::skip]
    pub const SQ_DISPLAY: [&'static str; Self::N_SQUARES] = [
        "a1", "b1", "c1", "d1", "e1", "f1", "g1", "h1",
        "a2", "b2", "c2", "d2", "e2", "f2", "g2", "h2",
        "a3", "b3", "c3", "d3", "e3", "f3", "g3", "h3",
        "a4", "b4", "c4", "d4", "e4", "f4", "g4", "h4",
        "a5", "b5", "c5", "d5", "e5", "f5", "g5", "h5",
        "a6", "b6", "c6", "d6", "e6", "f6", "g6", "h6",
        "a7", "b7", "c7", "d7", "e7", "f7", "g7", "h7",
        "a8", "b8", "c8", "d8", "e8", "f8", "g8", "h8"];
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
pub enum Direction {
    NorthNorth = 16,
    North = 8,
    South = -8,
    SouthSouth = -16,
    East = 1,
    West = -1,
    NorthEast = 9,
    NorthWest = 7,
    SouthEast = -7,
    SouthWest = -9,
}

impl Direction {
    pub fn relative(self, c: Color) -> Direction {
        if c == Color::White {
            return self;
        }
        Direction::from(-(self as i8))
    }
}

impl From<i8> for Direction {
    #[inline(always)]
    fn from(n: i8) -> Self {
        unsafe { std::mem::transmute::<i8, Self>(n) }
    }
}
