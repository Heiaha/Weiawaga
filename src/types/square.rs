use super::bitboard::BitBoard;
use super::color::Color;
use super::diagonal::{AntiDiagonal, Diagonal};
use super::file::File;
use super::rank::Rank;
use std::iter::Step;
use std::mem::transmute;
use std::ops::*;

pub const N_SQUARES: usize = 64;

pub const SQUARES_BB: [BitBoard; N_SQUARES + 1] = [
    BitBoard(1 << 0),  BitBoard(1 << 1),  BitBoard(1 << 2),  BitBoard(1 << 3),
    BitBoard(1 << 4),  BitBoard(1 << 5),  BitBoard(1 << 6),  BitBoard(1 << 7),
    BitBoard(1 << 8),  BitBoard(1 << 9),  BitBoard(1 << 10), BitBoard(1 << 11),
    BitBoard(1 << 12), BitBoard(1 << 13), BitBoard(1 << 14), BitBoard(1 << 15),
    BitBoard(1 << 16), BitBoard(1 << 17), BitBoard(1 << 18), BitBoard(1 << 19),
    BitBoard(1 << 20), BitBoard(1 << 21), BitBoard(1 << 22), BitBoard(1 << 23),
    BitBoard(1 << 24), BitBoard(1 << 25), BitBoard(1 << 26), BitBoard(1 << 27),
    BitBoard(1 << 28), BitBoard(1 << 29), BitBoard(1 << 30), BitBoard(1 << 31),
    BitBoard(1 << 32), BitBoard(1 << 33), BitBoard(1 << 34), BitBoard(1 << 35),
    BitBoard(1 << 36), BitBoard(1 << 37), BitBoard(1 << 38), BitBoard(1 << 39),
    BitBoard(1 << 40), BitBoard(1 << 41), BitBoard(1 << 42), BitBoard(1 << 43),
    BitBoard(1 << 44), BitBoard(1 << 45), BitBoard(1 << 46), BitBoard(1 << 47),
    BitBoard(1 << 48), BitBoard(1 << 49), BitBoard(1 << 50), BitBoard(1 << 51),
    BitBoard(1 << 52), BitBoard(1 << 53), BitBoard(1 << 54), BitBoard(1 << 55),
    BitBoard(1 << 56), BitBoard(1 << 57), BitBoard(1 << 58), BitBoard(1 << 59),
    BitBoard(1 << 60), BitBoard(1 << 61), BitBoard(1 << 62), BitBoard(1 << 63),
    BitBoard(0)
];

pub const SQ_DISPLAY: [&str; N_SQUARES] = [
    "a1", "b1", "c1", "d1", "e1", "f1", "g1", "h1",
    "a2", "b2", "c2", "d2", "e2", "f2", "g2", "h2",
    "a3", "b3", "c3", "d3", "e3", "f3", "g3", "h3",
    "a4", "b4", "c4", "d4", "e4", "f4", "g4", "h4",
    "a5", "b5", "c5", "d5", "e5", "f5", "g5", "h5",
    "a6", "b6", "c6", "d6", "e6", "f6", "g6", "h6",
    "a7", "b7", "c7", "d7", "e7", "f7", "g7", "h7",
    "a8", "b8", "c8", "d8", "e8", "f8", "g8", "h8"];

pub const SQ_DISPLAY_ORDER: [u8; N_SQUARES] = [
    56, 57, 58, 59, 60, 61, 62, 63,
    48, 49, 50, 51, 52, 53, 54, 55,
    40, 41, 42, 43, 44, 45, 46, 47,
    32, 33, 34, 35, 36, 37, 38, 39,
    24, 25, 26, 27, 28, 29, 30, 31,
    16, 17, 18, 19, 20, 21, 22, 23,
    8,  9,  10, 11, 12, 13, 14, 15,
    0,  1,   2,  3,  4,  5,  6,  7];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
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

impl SQ {
    pub fn encode(rank: Rank, file: File) -> Self {
        Self::from(((rank as u8) << 3) + (file as u8))
    }

    #[inline(always)]
    pub fn bb(self) -> BitBoard {
        SQUARES_BB[self as usize]
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
    pub fn square_mirror(self) -> SQ {
        SQ::from(self as u8 ^ 0x38)
    }

    #[inline]
    pub fn forward_ranks_bb(self, color: Color) -> BitBoard {
        return if color == Color::White {
            !Rank::Rank1.bb() << 8 * self.rank().relative(color) as u32
        } else {
            !Rank::Rank8.bb() >> 8 * self.rank().relative(color) as u32
        };
    }

    pub fn forward_files_bb(self, color: Color) -> BitBoard {
        return self.file().bb() & self.forward_ranks_bb(color);
    }

    pub fn get_ray(self, dir: Direction) -> BitBoard {
        return match dir {
            Direction::North => BitBoard(0x0101010101010100) << self as u32,
            Direction::South => BitBoard(0x0080808080808080) >> (63 - self as u32),
            Direction::East => {
                BitBoard(2) * ((BitBoard(1) << (self as u32 | 7)) - (BitBoard(1) << self as u32))
            }
            Direction::West => (BitBoard(1) << self as u32) - (BitBoard(1) << (self as u32 & 56)),
            Direction::NorthEast => {
                BitBoard(0x8040201008040200).shift(Direction::East, self.file() as u32)
                    << (self.rank() as u32 * 8)
            }
            Direction::NorthWest => {
                BitBoard(0x0102040810204000).shift(Direction::West, 7 - self.file() as u32)
                    << self.rank() as u32 * 8
            }
            Direction::SouthEast => {
                BitBoard(0x0002040810204080).shift(Direction::East, self.file() as u32)
                    >> ((7 - self.rank() as u32) * 8)
            }
            Direction::SouthWest => {
                BitBoard(0x0040201008040201).shift(Direction::West, 7 - self.file() as u32)
                    >> ((7 - self.rank() as u32) * 8)
            }
            _ => BitBoard::ZERO,
        };
    }
}

impl Direction {
    pub fn relative(self, c: Color) -> Direction {
        if c == Color::White {
            return self;
        }
        Direction::from(-(self as i8))
    }
}

//////////////////////////////////////////////
// Trait Implementations
//////////////////////////////////////////////

impl From<u8> for SQ {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { transmute::<u8, Self>(n) }
    }
}

impl From<&str> for SQ {
    fn from(name: &str) -> Self {
        for (i, &sq_name) in SQ_DISPLAY.iter().enumerate() {
            if name == sq_name {
                return Self::from(i as u8);
            }
        }
        SQ::None
    }
}

impl From<i8> for Direction {
    #[inline(always)]
    fn from(n: i8) -> Self {
        unsafe { transmute::<i8, Self>(n) }
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

impl Step for SQ {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        if start <= &SQ::H8 && end <= &SQ::H8 {
            Some((*end as usize - *start as usize) as usize)
        } else {
            None
        }
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        if start as usize + count > SQ::H8 as usize {
            return None;
        }
        Some(Self::from(start as u8 + count as u8))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        if start as usize - count > SQ::H8 as usize {
            return None;
        }
        Some(Self::from(start as u8 - count as u8))
    }
}

impl ToString for SQ {
    fn to_string(&self) -> String {
        SQ_DISPLAY[*self as usize].to_string()
    }
}
