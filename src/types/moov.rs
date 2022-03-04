use super::square::*;
use crate::types::piece::*;
use std::fmt;

pub type MoveInt = u16;

#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct Move(MoveInt);

impl Move {
    #[inline(always)]
    pub fn new(from_sq: SQ, to_square: SQ, flags: MoveFlags) -> Self {
        Self(((flags as MoveInt) << 12) | ((from_sq as MoveInt) << 6) | (to_square as MoveInt))
    }

    #[inline(always)]
    pub fn to_sq(&self) -> SQ {
        SQ::from((self.0 & 0x3f) as u8)
    }

    #[inline(always)]
    pub fn from_sq(&self) -> SQ {
        SQ::from(((self.0 >> 6) & 0x3f) as u8)
    }

    #[inline(always)]
    pub fn flags(&self) -> MoveFlags {
        MoveFlags::from(((self.0 >> 12) & 0xf) as u8)
    }

    #[inline(always)]
    pub fn is_quiet(&self) -> bool {
        (self.0 >> 12) & 0b1100 == 0
    }

    #[inline(always)]
    pub fn is_capture(&self) -> bool {
        ((self.0 >> 12) & 0b0100) != 0
    }

    #[inline(always)]
    pub fn is_ep(&self) -> bool {
        self.flags() == MoveFlags::EnPassant
    }

    #[inline(always)]
    pub fn promotion(&self) -> PieceType {
        match self.flags() {
            MoveFlags::PrKnight | MoveFlags::PcKnight => PieceType::Knight,
            MoveFlags::PrBishop | MoveFlags::PcBishop => PieceType::Bishop,
            MoveFlags::PrRook | MoveFlags::PcRook => PieceType::Rook,
            MoveFlags::PrQueen | MoveFlags::PcQueen => PieceType::Queen,
            _ => PieceType::None,
        }
    }

    #[inline(always)]
    pub fn is_castling(&self) -> bool {
        matches!(self.flags(), MoveFlags::OO | MoveFlags::OOO)
    }
}

impl From<MoveInt> for Move {
    #[inline(always)]
    fn from(m: MoveInt) -> Self {
        Self(m)
    }
}

impl Into<MoveInt> for Move {
    #[inline(always)]
    fn into(self) -> MoveInt {
        self.0
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.from_sq().to_string(),
            self.to_sq().to_string(),
            self.promotion().to_string()
        )
    }
}

#[derive(Debug, PartialEq)]
pub enum MoveFlags {
    Quiet = 0b0000,
    DoublePush = 0b0001,
    OO = 0b0010,
    OOO = 0b0011,
    Capture = 0b0100,
    EnPassant = 0b0101,
    PrKnight = 0b1000,
    PrBishop = 0b1001,
    PrRook = 0b1010,
    PrQueen = 0b1011,
    PcKnight = 0b1100,
    PcBishop = 0b1101,
    PcRook = 0b1110,
    PcQueen = 0b1111,
}

impl From<u8> for MoveFlags {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}
