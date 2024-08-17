use super::bitboard::*;
use super::color::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rank {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
}

impl Rank {
    pub fn bb(self) -> Bitboard {
        Self::RANK_BB[self as usize]
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn relative(self, c: Color) -> Self {
        Self::from((self as u8) ^ (c as u8 * 7))
    }
}

impl From<u8> for Rank {
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}

impl Rank {
    pub const N_RANKS: usize = 8;
    const RANK_BB: [Bitboard; Self::N_RANKS] = [
        B!(0x0000_0000_0000_00FF),
        B!(0x0000_0000_0000_FF00),
        B!(0x0000_0000_00FF_0000),
        B!(0x0000_0000_FF00_0000),
        B!(0x0000_00FF_0000_0000),
        B!(0x0000_FF00_0000_0000),
        B!(0x00FF_0000_0000_0000),
        B!(0xFF00_0000_0000_0000),
    ];
}
