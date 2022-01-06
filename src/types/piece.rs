use super::color::*;
use std::convert::TryFrom;
use std::iter::Step;
use std::mem::transmute;

pub const N_PIECES: usize = 15;
pub const N_PIECE_TYPES: usize = 6;

#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Debug)]
pub enum Piece {
    WhitePawn = 0,
    WhiteKnight = 1,
    WhiteBishop = 2,
    WhiteRook = 3,
    WhiteQueen = 4,
    WhiteKing = 5,
    BlackPawn = 8,
    BlackKnight = 9,
    BlackBishop = 10,
    BlackRook = 11,
    BlackQueen = 12,
    BlackKing = 13,
    None = 14,
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

impl Piece {
    #[inline(always)]
    pub fn index(self) -> usize {
        self as usize
    }

    #[inline(always)]
    pub fn flip(self) -> Piece {
        Piece::from(self as u8 ^ 0b1000)
    }

    #[inline(always)]
    pub fn type_of(self) -> PieceType {
        PieceType::from(self as u8 & 0b111)
    }

    #[inline(always)]
    pub fn color_of(self) -> Color {
        Color::from((self as u8 & 0b1000) >> 3)
    }

    #[inline(always)]
    pub fn make_piece(color: Color, pt: PieceType) -> Self {
        Self::from(((color as u8) << 3) + pt as u8)
    }

    pub fn uci(self) -> char {
        Self::PIECE_STR.chars().nth(self.index()).unwrap()
    }

    pub fn symbol(self) -> char {
        Self::PIECE_STR.chars().nth(self.index()).unwrap()
    }
}

impl PieceType {
    #[inline(always)]
    pub fn index(self) -> usize {
        self as usize
    }
}

impl From<u8> for Piece {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { transmute::<u8, Self>(n) }
    }
}

impl From<u8> for PieceType {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { transmute::<u8, Self>(n) }
    }
}

impl Step for Piece {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        Some(*end as usize - *start as usize)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        Some(Self::from(start as u8 + count as u8))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        Some(Self::from(start as u8 - count as u8))
    }
}

impl Step for PieceType {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        Some(*end as usize - *start as usize)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        Some(Self::from(start as u8 + count as u8))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        Some(Self::from(start as u8 - count as u8))
    }
}

impl TryFrom<char> for Piece {
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        if Self::PIECE_STR.contains(value) {
            return Ok(Self::from(
                Self::PIECE_STR.chars().position(|c| c == value).unwrap() as u8,
            ));
        }
        Err("Piece symbols should be one of \"KQRBNPkqrbnp\"")
    }
}

impl Piece {
    const PIECE_STR: &'static str = "PNBRQK  pnbrqk ";
    const SYMBOL_STR: &'static str = "♙♘♗♖♕♔  ♟♞♝♜♛♚";
}
