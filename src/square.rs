use super::bitboard::*;
use super::piece::*;
use super::types::*;
use std::fmt;
use std::ops::{Add, Sub};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
#[rustfmt::skip]
#[repr(u8)]
pub enum SQ {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
}

impl SQ {
    pub fn encode(rank: Rank, file: File) -> Self {
        Self::from(((rank as u8) << 3) + (file as u8))
    }

    pub fn bb(self) -> Bitboard {
        Self::SQUARES_BB[self]
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn rank(self) -> Rank {
        Rank::from(self as u8 >> 3)
    }

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

    pub fn square_mirror(self) -> Self {
        Self::from(self as u8 ^ 0x38)
    }

    pub fn relative(self, c: Color) -> Self {
        match c {
            Color::White => self,
            Color::Black => self.square_mirror(),
        }
    }

    pub fn iter(start: Self, end: Self) -> impl Iterator<Item = Self> {
        (start as u8..=end as u8).map(Self::from)
    }
}

impl Add<Direction> for SQ {
    type Output = Self;

    fn add(self, dir: Direction) -> Self {
        Self::from((self as u8).wrapping_add(dir as u8))
    }
}

impl Sub<Direction> for SQ {
    type Output = Self;

    fn sub(self, dir: Direction) -> Self {
        Self::from((self as u8).wrapping_sub(dir as u8))
    }
}

impl From<u8> for SQ {
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
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
        let sq = Self::from(
            Self::SQ_DISPLAY
                .into_iter()
                .position(|potential_sq_str| *potential_sq_str == value)
                .ok_or("Invalid square.")? as u8,
        );
        Ok(sq)
    }
}

impl Into<usize> for SQ {
    fn into(self) -> usize {
        self.index()
    }
}

impl SQ {
    pub const N_SQUARES: usize = 64;

    #[rustfmt::skip]
    const SQUARES_BB: SQMap<Bitboard> = SQMap::new([
        B!(1 << 0),  B!(1 << 1),  B!(1 << 2),  B!(1 << 3),  B!(1 << 4),  B!(1 << 5),  B!(1 << 6),  B!(1 << 7),
        B!(1 << 8),  B!(1 << 9),  B!(1 << 10), B!(1 << 11), B!(1 << 12), B!(1 << 13), B!(1 << 14), B!(1 << 15),
        B!(1 << 16), B!(1 << 17), B!(1 << 18), B!(1 << 19), B!(1 << 20), B!(1 << 21), B!(1 << 22), B!(1 << 23),
        B!(1 << 24), B!(1 << 25), B!(1 << 26), B!(1 << 27), B!(1 << 28), B!(1 << 29), B!(1 << 30), B!(1 << 31),
        B!(1 << 32), B!(1 << 33), B!(1 << 34), B!(1 << 35), B!(1 << 36), B!(1 << 37), B!(1 << 38), B!(1 << 39),
        B!(1 << 40), B!(1 << 41), B!(1 << 42), B!(1 << 43), B!(1 << 44), B!(1 << 45), B!(1 << 46), B!(1 << 47),
        B!(1 << 48), B!(1 << 49), B!(1 << 50), B!(1 << 51), B!(1 << 52), B!(1 << 53), B!(1 << 54), B!(1 << 55),
        B!(1 << 56), B!(1 << 57), B!(1 << 58), B!(1 << 59), B!(1 << 60), B!(1 << 61), B!(1 << 62), B!(1 << 63),
    ]);

    #[rustfmt::skip]
    pub const SQ_DISPLAY: SQMap<&'static str> = SQMap::new([
        "a1", "b1", "c1", "d1", "e1", "f1", "g1", "h1",
        "a2", "b2", "c2", "d2", "e2", "f2", "g2", "h2",
        "a3", "b3", "c3", "d3", "e3", "f3", "g3", "h3",
        "a4", "b4", "c4", "d4", "e4", "f4", "g4", "h4",
        "a5", "b5", "c5", "d5", "e5", "f5", "g5", "h5",
        "a6", "b6", "c6", "d6", "e6", "f6", "g6", "h6",
        "a7", "b7", "c7", "d7", "e7", "f7", "g7", "h7",
        "a8", "b8", "c8", "d8", "e8", "f8", "g8", "h8"
    ]);
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
        match c {
            Color::White => self,
            Color::Black => Direction::from(-(self as i8)),
        }
    }
}

impl From<i8> for Direction {
    fn from(n: i8) -> Self {
        unsafe { std::mem::transmute::<i8, Self>(n) }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Rank {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
}

impl Rank {
    pub fn bb(self) -> Bitboard {
        Self::RANK_BB[self as usize]
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn relative(self, c: Color) -> Self {
        Self::from((self as u8) ^ (c as u8 * 7))
    }
}

impl From<u8> for Rank {
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}

impl Into<usize> for Rank {
    fn into(self) -> usize {
        self.index()
    }
}

impl Rank {
    pub const N_RANKS: usize = 8;
    const RANK_BB: RankMap<Bitboard> = RankMap::new([
        B!(0x0000_0000_0000_00FF),
        B!(0x0000_0000_0000_FF00),
        B!(0x0000_0000_00FF_0000),
        B!(0x0000_0000_FF00_0000),
        B!(0x0000_00FF_0000_0000),
        B!(0x0000_FF00_0000_0000),
        B!(0x00FF_0000_0000_0000),
        B!(0xFF00_0000_0000_0000),
    ]);
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
#[repr(u8)]
pub enum File {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
}

impl File {
    pub fn bb(self) -> Bitboard {
        Self::FILE_BB[self as usize]
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

impl From<u8> for File {
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}

impl Into<usize> for File {
    fn into(self) -> usize {
        self.index()
    }
}

impl File {
    pub const N_FILES: usize = 8;
    const FILE_BB: FileMap<Bitboard> = FileMap::new([
        B!(0x0101_0101_0101_0101),
        B!(0x0202_0202_0202_0202),
        B!(0x0404_0404_0404_0404),
        B!(0x0808_0808_0808_0808),
        B!(0x1010_1010_1010_1010),
        B!(0x2020_2020_2020_2020),
        B!(0x4040_4040_4040_4040),
        B!(0x8080_8080_8080_8080),
    ]);
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Diagonal {
    H1H1,
    G1H2,
    F1H3,
    E1H4,
    D1H5,
    C1H6,
    B1H7,
    H8A1,
    G8A2,
    F8A3,
    E8A4,
    D8A5,
    C8A6,
    B8A7,
    A8A8,
}

impl Diagonal {
    pub fn bb(self) -> Bitboard {
        Self::DIAGONAL_BB[self as usize]
    }
}

impl From<u8> for Diagonal {
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}

impl Diagonal {
    pub const N_DIAGONALS: usize = 15;
    const DIAGONAL_BB: DiagonalMap<Bitboard> = DiagonalMap::new([
        B!(0x0000_0000_0000_0080),
        B!(0x0000_0000_0000_8040),
        B!(0x0000_0000_0080_4020),
        B!(0x0000_0000_8040_2010),
        B!(0x0000_0080_4020_1008),
        B!(0x0000_8040_2010_0804),
        B!(0x0080_4020_1008_0402),
        B!(0x8040_2010_0804_0201),
        B!(0x4020_1008_0402_0100),
        B!(0x2010_0804_0201_0000),
        B!(0x1008_0402_0100_0000),
        B!(0x0804_0201_0000_0000),
        B!(0x0402_0100_0000_0000),
        B!(0x0201_0000_0000_0000),
        B!(0x0100_0000_0000_0000),
    ]);
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum AntiDiagonal {
    A1A1,
    B1A2,
    C1A3,
    D1A4,
    E1A5,
    F1A6,
    G1A7,
    H1A8,
    B8H2,
    C8H3,
    D8H4,
    E8H5,
    F8H6,
    G8H7,
    H8H8,
}

impl AntiDiagonal {
    pub fn bb(self) -> Bitboard {
        Self::ANTIDIAGONAL_BB[self as usize]
    }
}

impl From<u8> for AntiDiagonal {
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}

impl AntiDiagonal {
    pub const N_ANTIDIAGONALS: usize = 15;
    const ANTIDIAGONAL_BB: DiagonalMap<Bitboard> = DiagonalMap::new([
        B!(0x0000_0000_0000_0001),
        B!(0x0000_0000_0000_0102),
        B!(0x0000_0000_0001_0204),
        B!(0x0000_0000_0102_0408),
        B!(0x0000_0001_0204_0810),
        B!(0x0000_0102_0408_1020),
        B!(0x0001_0204_0810_2040),
        B!(0x0102_0408_1020_4080),
        B!(0x0204_0810_2040_8000),
        B!(0x0408_1020_4080_0000),
        B!(0x0810_2040_8000_0000),
        B!(0x1020_4080_0000_0000),
        B!(0x2040_8000_0000_0000),
        B!(0x4080_0000_0000_0000),
        B!(0x8000_0000_0000_0000),
    ]);
}
