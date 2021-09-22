use super::square::SQ;
use std::mem::transmute;


#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Moove {
    m: u16,
    score: u16
}

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

impl Moove {
    pub const FLAG_PROMOTION: u8 = 0b1000;
    pub const FLAG_NULL: u8 = 0b1001;

    #[inline(always)]
    pub fn new(from_sq: SQ, to_square: SQ, flags: MoveFlags) -> Self {
        Moove {
            m: ((flags as u16) << 12) | ((from_sq as u16) << 6) | (to_square as u16),
              score: 0
        }
    }

    #[inline(always)]
    pub fn empty() -> Self {
        Moove {
            m: 0,
            score: 0
        }
    }

    #[inline(always)]
    pub fn to_sq(self) -> SQ {
        SQ::from((self.m & 0x3f) as u8)
    }

    #[inline(always)]
    pub fn from_sq(self) -> SQ {
        SQ::from(((self.m >> 6) & 0x3f) as u8)
    }

    #[inline(always)]
    pub fn flags(self) -> MoveFlags {
        MoveFlags::from(((self.m >> 12) & 0xf) as u8)
    }

    #[inline(always)]
    pub fn moove(self) -> u16 {
        self.m
    }

    #[inline(always)]
    pub fn is_capture(&self) -> bool {
        ((self.m >> 12) & MoveFlags::Capture as u16) != 0
    }

    #[inline(always)]
    pub fn set_score(&mut self, score: u16) {
        self.score = score;
    }

}

impl From<u16> for Moove {

    #[inline(always)]
    fn from(m: u16) -> Self {
        Moove {
            m,
            score: 0
        }
    }
}

impl From<u8> for MoveFlags {

    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe {
            transmute::<u8, Self>(n)
        }
    }
}

impl ToString for Moove {
    fn to_string(&self) -> String {
        let mut uci = "".to_owned();
        uci.push_str(&*self.from_sq().to_string());
        uci.push_str(&*self.to_sq().to_string());
        uci.to_owned()
    }
}