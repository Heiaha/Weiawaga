use super::bitboard::*;
use super::color::*;
use std::mem::transmute;
use std::ops::*;

pub static RANK_BB: [BitBoard; 8] = [
    B!(0x0000_0000_0000_00FF),
    B!(0x0000_0000_0000_FF00),
    B!(0x0000_0000_00FF_0000),
    B!(0x0000_0000_FF00_0000),
    B!(0x0000_00FF_0000_0000),
    B!(0x0000_FF00_0000_0000),
    B!(0x00FF_0000_0000_0000),
    B!(0xFF00_0000_0000_0000),
];

const RANK_DISPLAYS: [char; 8] = ['1', '2', '3', '4', '5', '6', '7', '8'];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
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
    #[inline(always)]
    pub fn bb(self) -> BitBoard {
        RANK_BB[self as usize]
    }

    #[inline(always)]
    pub fn relative(self, c: Color) -> Rank {
        Rank::from((self as u8) ^ (c as u8 * 7))
    }

    #[inline(never)]
    pub fn to_char(&self) -> char {
        RANK_DISPLAYS[*self as usize]
    }
}

impl From<u8> for Rank {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { transmute::<u8, Self>(n) }
    }
}

impl Add for Rank {
    type Output = Self;

    #[inline(always)]
    fn add(self, other: Self) -> Self::Output {
        Rank::from(self as u8 + other as u8)
    }
}

impl Sub for Rank {
    type Output = Self;

    #[inline(always)]
    fn sub(self, other: Self) -> Self::Output {
        Rank::from(self as u8 - other as u8)
    }
}
