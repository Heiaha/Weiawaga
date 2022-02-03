use super::square::*;
use crate::search::move_sorter::*;
use std::fmt;
use std::fmt::Formatter;
use std::mem::transmute;

pub type MoveInt = u16;

#[derive(Copy, Clone, Default, Debug)]
pub struct Move {
    m: MoveInt,
    score: SortScore,
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

impl Move {
    #[inline(always)]
    pub fn new(from_sq: SQ, to_square: SQ, flags: MoveFlags) -> Self {
        Self {
            m: ((flags as MoveInt) << 12) | ((from_sq as MoveInt) << 6) | (to_square as MoveInt),
            score: 0,
        }
    }

    #[inline(always)]
    pub fn empty() -> Self {
        Self { m: 0, score: 0 }
    }

    #[inline(always)]
    pub fn to_sq(&self) -> SQ {
        SQ::from((self.m & 0x3f) as u8)
    }

    #[inline(always)]
    pub fn from_sq(&self) -> SQ {
        SQ::from(((self.m >> 6) & 0x3f) as u8)
    }

    #[inline(always)]
    pub fn flags(&self) -> MoveFlags {
        MoveFlags::from(((self.m >> 12) & 0xf) as u8)
    }

    #[inline(always)]
    pub fn score(&self) -> SortScore {
        self.score
    }

    #[inline(always)]
    pub fn moove(&self) -> MoveInt {
        self.m
    }

    #[inline(always)]
    pub fn is_quiet(&self) -> bool {
        ((self.m >> 12) & 0b1100) == 0
    }

    #[inline(always)]
    pub fn is_capture(&self) -> bool {
        ((self.m >> 12) & 0b0100) != 0
    }

    #[inline(always)]
    pub fn is_promotion(&self) -> bool {
        ((self.m >> 12) & 0b1000) != 0
    }

    #[inline(always)]
    pub fn is_castling(&self) -> bool {
        matches!(self.flags(), MoveFlags::OO | MoveFlags::OOO)
    }

    #[inline(always)]
    pub fn set_score(&mut self, score: SortScore) {
        self.score = score;
    }

    #[inline(always)]
    pub fn add_to_score(&mut self, score: SortScore) {
        self.score += score;
    }
}

impl From<MoveInt> for Move {
    #[inline(always)]
    fn from(m: MoveInt) -> Self {
        Self { m, score: 0 }
    }
}

impl From<u8> for MoveFlags {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { transmute::<u8, Self>(n) }
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut uci = String::new();
        uci.push_str(&*self.from_sq().to_string());
        uci.push_str(&*self.to_sq().to_string());
        match self.flags() {
            MoveFlags::PrKnight | MoveFlags::PcKnight => {
                uci.push('n');
            }
            MoveFlags::PrBishop | MoveFlags::PcBishop => {
                uci.push('b');
            }
            MoveFlags::PrRook | MoveFlags::PcRook => {
                uci.push('r');
            }
            MoveFlags::PrQueen | MoveFlags::PcQueen => {
                uci.push('q');
            }
            _ => {}
        }
        write!(f, "{}", uci)
    }
}

impl PartialEq for Move {
    fn eq(&self, other: &Self) -> bool {
        self.m == other.m
    }
}

impl Move {
    pub const NULL: Self = Self { m: 4160, score: 0 };
}
