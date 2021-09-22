use crate::types::bitboard::BitBoard;
use std::mem::transmute;
use std::ops::*;

pub static FILE_BB: [BitBoard; 8] = [
    B!(0x0101_0101_0101_0101),
    B!(0x0202_0202_0202_0202),
    B!(0x0404_0404_0404_0404),
    B!(0x0808_0808_0808_0808),
    B!(0x1010_1010_1010_1010),
    B!(0x2020_2020_2020_2020),
    B!(0x4040_4040_4040_4040),
    B!(0x8080_8080_8080_8080),
];

const FILE_DISPLAYS: [char; 8] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
pub enum File {
    FileA,
    FileB,
    FileC,
    FileD,
    FileE,
    FileF,
    FileG,
    FileH,
}

impl File {
    #[inline(always)]
    pub fn bb(self) -> BitBoard {
        FILE_BB[self as usize]
    }

    #[inline(never)]
    pub fn to_char(&self) -> char {
        FILE_DISPLAYS[*self as usize]
    }
}

impl From<u8> for File {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { transmute::<u8, Self>(n) }
    }
}

impl Add for File {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        File::from(self as u8 + other as u8)
    }
}

impl Sub for File {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        File::from(self as u8 - other as u8)
    }
}
