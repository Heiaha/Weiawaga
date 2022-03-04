use super::color::*;
use std::convert::TryFrom;
use std::fmt;

#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Debug)]
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
    None = 0b1110,
}

impl Piece {
    #[inline(always)]
    pub fn index(self) -> usize {
        self as usize - 2 * self.color_of().index()
    }

    #[inline(always)]
    pub fn flip(self) -> Piece {
        Self::from(self as u8 ^ 0b1000)
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

    // Use this iterator pattern for Piece, PieceType, and Bitboard iterator for SQ
    // until we can return to Step implementation once it's stabilized.
    // https://github.com/rust-lang/rust/issues/42168
    #[inline(always)]
    pub fn iter(start: Self, end: Self) -> impl Iterator<Item = Self> {
        (start as u8..=end as u8)
            .filter(|n| !matches!(n, 0b0110 | 0b0111)) // Skip over 6 and 7, as they're not assigned to a piece so as to align color bits
            .map(Self::from)
    }
}

impl From<u8> for Piece {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
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

impl Default for Piece {
    fn default() -> Self {
        Self::None
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
                .unwrap()
                .to_string()
                .replace(" ", "")
        )
    }
}

impl Piece {
    pub const N_PIECES: usize = 13;
    const PIECE_STR: &'static str = "PNBRQK  pnbrqk ";
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Debug)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
    None,
}

impl PieceType {
    #[inline(always)]
    pub fn index(self) -> usize {
        self as usize
    }

    #[inline(always)]
    pub fn iter(start: Self, end: Self) -> impl Iterator<Item = Self> {
        (start as u8..=end as u8).map(|n| Self::from(n))
    }
}

impl From<u8> for PieceType {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}

impl Default for PieceType {
    fn default() -> Self {
        Self::None
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
                .unwrap()
                .to_string()
                .replace(" ", "")
        )
    }
}

impl PieceType {
    pub const N_PIECE_TYPES: usize = 6;
    pub const PIECE_TYPE_STR: &'static str = "pnbrqk ";
}
