use super::traits::*;
use std::fmt;
use std::ops::Not;

#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Debug)]
#[repr(u8)]
pub enum Piece {
    WhitePawn = 0b0000,
    WhiteKnight = 0b0001,
    WhiteBishop = 0b0010,
    WhiteRook = 0b0011,
    WhiteQueen = 0b0100,
    WhiteKing = 0b0101,
    BlackPawn = 0b1000,
    BlackKnight = 0b1001,
    BlackBishop = 0b1010,
    BlackRook = 0b1011,
    BlackQueen = 0b1100,
    BlackKing = 0b1101,
}

impl Piece {
    pub fn index(self) -> usize {
        self as usize - 2 * self.color_of().index()
    }

    pub fn type_of(self) -> PieceType {
        PieceType::from(self as u8 & 0b111)
    }

    pub fn color_of(self) -> Color {
        Color::from((self as u8 & 0b1000) >> 3)
    }

    pub fn make_piece(color: Color, pt: PieceType) -> Self {
        Self::from(((color as u8) << 3) + pt as u8)
    }

    // Use this iterator pattern for Piece, PieceType, and Bitboard iterator for SQ
    // until we can return to Step implementation once it's stabilized.
    // https://github.com/rust-lang/rust/issues/42168
    pub fn iter(start: Self, end: Self) -> impl Iterator<Item = Self> {
        (start as u8..=end as u8)
            .filter(|n| !matches!(n, 0b0110 | 0b0111)) // Skip over 6 and 7, as they're not assigned to a piece so as to align color bits
            .map(Self::from)
    }
}

impl Mirror for Piece {
    fn mirror(&self) -> Self {
        Self::from(*self as u8 ^ 0b1000)
    }
}

impl From<u8> for Piece {
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}

impl TryFrom<char> for Piece {
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        Self::PIECE_STR
            .chars()
            .position(|c| c == value)
            .map(|x| Self::from(x as u8))
            .ok_or("Piece symbols should be one of \"KQRBNPkqrbnp\"")
    }
}

impl Into<usize> for Piece {
    fn into(self) -> usize {
        self.index()
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            Self::PIECE_STR
                .chars()
                .nth(*self as usize)
                .expect("Piece symbol should be valid.")
        )
    }
}

impl Piece {
    pub const N_PIECES: usize = 12;
    const PIECE_STR: &'static str = "PNBRQK  pnbrqk";
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Debug)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceType {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn iter(start: Self, end: Self) -> impl Iterator<Item = Self> {
        (start as u8..=end as u8).map(Self::from)
    }
}

impl From<u8> for PieceType {
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}

impl Into<usize> for PieceType {
    fn into(self) -> usize {
        self.index()
    }
}

impl fmt::Display for PieceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            Self::PIECE_TYPE_STR
                .chars()
                .nth(*self as usize)
                .expect("PieceType symbol should be valid.")
        )
    }
}

impl PieceType {
    pub const N_PIECE_TYPES: usize = 6;
    pub const PIECE_TYPE_STR: &'static str = "pnbrqk";
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn factor(&self) -> i32 {
        match *self {
            Self::White => 1,
            Self::Black => -1,
        }
    }
}

impl From<u8> for Color {
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}

impl Into<usize> for Color {
    fn into(self) -> usize {
        self.index()
    }
}

impl Not for Color {
    type Output = Color;

    fn not(self) -> Self {
        Color::from((self as u8) ^ 1)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::White => "w",
                Self::Black => "b",
            }
        )
    }
}

impl TryFrom<char> for Color {
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'w' => Ok(Self::White),
            'b' => Ok(Self::Black),
            _ => Err("Color must be either 'w' or 'b'."),
        }
    }
}

impl Color {
    pub const N_COLORS: usize = 2;
}
