use super::square::SQ;
use crate::search::move_scorer::SortScore;
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
        Move { m: ((flags as MoveInt) << 12) | ((from_sq as MoveInt) << 6) | (to_square as MoveInt),
               score: 0 }
    }

    #[inline(always)]
    pub fn empty() -> Self {
        Move { m: 0, score: 0 }
    }

    #[inline(always)]
    pub fn null() -> Self {
        Move::new(SQ::None, SQ::None, MoveFlags::Quiet)
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
    pub fn is_capture(&self) -> bool {
        ((self.m >> 12) & MoveFlags::Capture as MoveInt) != 0
    }

    #[inline(always)]
    pub fn is_promotion(&self) -> bool {
        ((self.m >> 12) & MoveFlags::PrKnight as MoveInt) != 0
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
        Move { m, score: 0 }
    }
}

impl From<u8> for MoveFlags {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { transmute::<u8, Self>(n) }
    }
}

impl ToString for Move {
    fn to_string(&self) -> String {
        let mut uci = "".to_owned();
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
        uci.to_owned()
    }
}

impl PartialEq for Move {
    fn eq(&self, other: &Self) -> bool {
        self.m == other.m
    }
}

impl Move {
    pub const NULL: Move = Move { m: 4160, score: 0 };
}
