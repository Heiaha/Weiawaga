use super::bitboard::*;
use std::ops::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
pub enum File {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
}

impl File {
    #[inline(always)]
    pub fn bb(self) -> Bitboard {
        Self::FILE_BB[self as usize]
    }

    #[inline(always)]
    pub fn index(self) -> usize {
        self as usize
    }
}

impl From<u8> for File {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}

impl Add for File {
    type Output = Self;

    #[inline(always)]
    fn add(self, other: Self) -> Self::Output {
        Self::from(self as u8 + other as u8)
    }
}

impl Sub for File {
    type Output = Self;

    #[inline(always)]
    fn sub(self, other: Self) -> Self::Output {
        Self::from(self as u8 - other as u8)
    }
}

impl File {
    pub const N_FILES: usize = 8;
    const FILE_BB: [Bitboard; Self::N_FILES] = [
        B!(0x0101_0101_0101_0101),
        B!(0x0202_0202_0202_0202),
        B!(0x0404_0404_0404_0404),
        B!(0x0808_0808_0808_0808),
        B!(0x1010_1010_1010_1010),
        B!(0x2020_2020_2020_2020),
        B!(0x4040_4040_4040_4040),
        B!(0x8080_8080_8080_8080),
    ];
}
