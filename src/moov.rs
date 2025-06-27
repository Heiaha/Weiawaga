use super::piece::*;
use super::square::*;
use std::fmt;
use std::num::NonZeroU16;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Move(NonZeroU16);

impl Move {
    pub fn new(from_sq: SQ, to_sq: SQ, flags: MoveFlags) -> Self {
        Self(
            NonZeroU16::new((flags as u16) << 12 | (from_sq as u16) << 6 | (to_sq as u16))
                .expect("MoveInt is zero."),
        )
    }

    pub fn to_sq(&self) -> SQ {
        SQ::from((self.0.get() & 0x3f) as u8)
    }

    pub fn from_sq(&self) -> SQ {
        SQ::from(((self.0.get() >> 6) & 0x3f) as u8)
    }

    pub fn squares(&self) -> (SQ, SQ) {
        (self.from_sq(), self.to_sq())
    }

    pub fn flags(&self) -> MoveFlags {
        MoveFlags::from(((self.0.get() >> 12) & 0xf) as u8)
    }

    pub fn move_int(&self) -> u16 {
        self.0.get()
    }

    pub fn is_quiet(&self) -> bool {
        (self.0.get() >> 12) & 0b1100 == 0
    }

    pub fn is_capture(&self) -> bool {
        (self.0.get() >> 12) & 0b0100 != 0
    }

    pub fn is_ep(&self) -> bool {
        self.flags() == MoveFlags::EnPassant
    }

    pub fn promotion(&self) -> Option<PieceType> {
        match self.flags() {
            MoveFlags::PrKnight | MoveFlags::PcKnight => Some(PieceType::Knight),
            MoveFlags::PrBishop | MoveFlags::PcBishop => Some(PieceType::Bishop),
            MoveFlags::PrRook | MoveFlags::PcRook => Some(PieceType::Rook),
            MoveFlags::PrQueen | MoveFlags::PcQueen => Some(PieceType::Queen),
            _ => None,
        }
    }

    pub fn is_castling(&self) -> bool {
        matches!(self.flags(), MoveFlags::OO | MoveFlags::OOO)
    }
}

impl From<u16> for Move {
    fn from(m: u16) -> Self {
        Self(NonZeroU16::new(m).expect("MoveInt is zero."))
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.from_sq(), self.to_sq())?;

        if let Some(promotion_pt) = self.promotion() {
            write!(f, "{promotion_pt}")?;
        }

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}
