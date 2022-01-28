use super::color::*;
use std::convert::TryFrom;
use std::mem::transmute;

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

    pub fn nn_index(&self) -> usize {
        return if self.color_of() == Color::White {
            *self as usize
        } else {
            *self as usize - 2
        };
    }
    pub fn uci(self) -> char {
        Self::PIECE_STR.chars().nth(self.index()).unwrap()
    }

    // Use this iterator pattern for Piece, PieceType, and Bitboard iterator for SQ
    // until we can return to Step implementation once it's stabilized.
    // https://github.com/rust-lang/rust/issues/42168
    #[inline(always)]
    pub fn iter(start: Self, end: Self) -> impl Iterator<Item = Self> {
        Self::ALL[start.index()..=end.index()]
            .iter()
            .copied()
            .filter(|x| *x != Self::None)
    }
}

impl From<u8> for Piece {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { transmute::<u8, Self>(n) }
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
    pub const N_PIECES: usize = 15;
    const PIECE_STR: &'static str = "PNBRQK  pnbrqk ";
    const ALL: [Self; Self::N_PIECES] = [
        Self::WhitePawn,
        Self::WhiteKnight,
        Self::WhiteBishop,
        Self::WhiteRook,
        Self::WhiteQueen,
        Self::WhiteKing,
        Self::None,
        Self::None,
        Self::BlackPawn,
        Self::BlackKnight,
        Self::BlackBishop,
        Self::BlackRook,
        Self::BlackQueen,
        Self::BlackKing,
        Self::None,
    ];
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
    #[inline(always)]
    pub fn index(self) -> usize {
        self as usize
    }

    #[inline(always)]
    pub fn iter(start: Self, end: Self) -> impl Iterator<Item = Self> {
        Self::ALL[start.index()..=end.index()].iter().copied()
    }
}

impl From<u8> for PieceType {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { transmute::<u8, Self>(n) }
    }
}

impl PieceType {
    pub const N_PIECE_TYPES: usize = 6;
    const ALL: [Self; Self::N_PIECE_TYPES] = [
        Self::Pawn,
        Self::Knight,
        Self::Bishop,
        Self::Rook,
        Self::Queen,
        Self::King,
    ];
}
