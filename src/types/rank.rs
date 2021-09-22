use crate::types::bitboard::BitBoard;
use crate::types::color::Color;
use std::mem::transmute;
use std::ops::*;

pub static RANK_BB: [BitBoard; 8] = [
    BitBoard(0x0000_0000_0000_00FF),
    BitBoard(0x0000_0000_0000_FF00),
    BitBoard(0x0000_0000_00FF_0000),
    BitBoard(0x0000_0000_FF00_0000),
    BitBoard(0x0000_00FF_0000_0000),
    BitBoard(0x0000_FF00_0000_0000),
    BitBoard(0x00FF_0000_0000_0000),
    BitBoard(0xFF00_0000_0000_0000),
];

pub static RANK_DISPLAYS: [char; 8] = ['1', '2', '3', '4', '5', '6', '7', '8'];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
pub enum Rank {
    Rank1 = 0,
    Rank2,
    Rank3,
    Rank4,
    Rank5,
    Rank6,
    Rank7,
    Rank8,
}

impl Rank {
    #[inline(always)]
    pub fn bb(self) -> BitBoard {
        RANK_BB[self as usize]
    }

    #[inline(always)]
    pub fn relative(self, c: Color) -> Rank {
        let r = (self as u8) ^ (c as u8 * 7);
        Rank::from(r)
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

    fn add(self, other: Self) -> Self::Output {
        Rank::from(self as u8 + other as u8)
    }
}

impl Sub for Rank {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Rank::from(self as u8 - other as u8)
    }
}
