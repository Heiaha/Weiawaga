use super::traits::*;
use super::types::*;
use std::fmt;
use std::ops::Not;

pub type PieceMap<T> = EnumMap<T, { Piece::COUNT }>;

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
    pub fn type_of(self) -> PieceType {
        PieceType::from_repr(self as u8 & 0b111)
    }

    pub fn color_of(self) -> Color {
        Color::from_repr((self as u8 & 0b1000) >> 3)
    }

    pub fn make_piece(color: Color, pt: PieceType) -> Self {
        Self::from(((color as u8) << 3) + pt as u8)
    }
}

impl Enumerable for Piece {
    const COUNT: usize = 12;

    // Discriminants skip 6 and 7 to keep the color bit aligned, so the dense
    // index and the discriminant differ for black pieces.
    fn from_repr(n: u8) -> Self {
        debug_assert!((n as usize) < Self::COUNT);
        Self::from(n + 2 * (n / 6))
    }

    fn index(&self) -> usize {
        *self as usize - 2 * self.color_of().index()
    }
}

impl Relative for Piece {
    fn vmirror(&self) -> Self {
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
    const PIECE_STR: &'static str = "PNBRQK  pnbrqk";
}

pub type PieceTypeMap<T> = EnumMap<T, { PieceType::COUNT }>;

#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Debug)]
#[repr(u8)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Enumerable for PieceType {
    const COUNT: usize = 6;
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
    pub const PIECE_TYPE_STR: &'static str = "pnbrqk";
}

pub type ColorMap<T> = EnumMap<T, { Color::COUNT }>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    White,
    Black,
}

impl Enumerable for Color {
    const COUNT: usize = 2;
}

impl Not for Color {
    type Output = Color;

    fn not(self) -> Self {
        Color::from_repr((self as u8) ^ 1)
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
