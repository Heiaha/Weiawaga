use super::bitboard::*;
use std::mem::transmute;

pub static DIAGONAL_BB: [BitBoard; 15] = [
    B!(0x0000_0000_0000_0080),
    B!(0x0000_0000_0000_8040),
    B!(0x0000_0000_0080_4020),
    B!(0x0000_0000_8040_2010),
    B!(0x0000_0080_4020_1008),
    B!(0x0000_8040_2010_0804),
    B!(0x0080_4020_1008_0402),
    B!(0x8040_2010_0804_0201),
    B!(0x4020_1008_0402_0100),
    B!(0x2010_0804_0201_0000),
    B!(0x1008_0402_0100_0000),
    B!(0x0804_0201_0000_0000),
    B!(0x0402_0100_0000_0000),
    B!(0x0201_0000_0000_0000),
    B!(0x0100_0000_0000_0000),
];

pub static ANTIDIAGONAL_BB: [BitBoard; 15] = [
    B!(0x0000_0000_0000_0001),
    B!(0x0000_0000_0000_0102),
    B!(0x0000_0000_0001_0204),
    B!(0x0000_0000_0102_0408),
    B!(0x0000_0001_0204_0810),
    B!(0x0000_0102_0408_1020),
    B!(0x0001_0204_0810_2040),
    B!(0x0102_0408_1020_4080),
    B!(0x0204_0810_2040_8000),
    B!(0x0408_1020_4080_0000),
    B!(0x0810_2040_8000_0000),
    B!(0x1020_4080_0000_0000),
    B!(0x2040_8000_0000_0000),
    B!(0x4080_0000_0000_0000),
    B!(0x8000_0000_0000_0000),
];

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Diagonal {
    H1H1,
    G1H2,
    F1H3,
    E1H4,
    D1H5,
    C1H6,
    B1H7,
    H8A1,
    G8A2,
    F8A3,
    E8A4,
    D8A5,
    C8A6,
    B8A7,
    A8A8,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AntiDiagonal {
    A1A1,
    B1A2,
    C1A3,
    D1A4,
    E1A5,
    F1A6,
    G1A7,
    H1A8,
    B8H2,
    C8H3,
    D8H4,
    E8H5,
    F8H6,
    G8H7,
    H8H8,
}

impl Diagonal {
    #[inline(always)]
    pub fn bb(self) -> BitBoard {
        DIAGONAL_BB[self as usize]
    }
}

impl AntiDiagonal {
    #[inline(always)]
    pub fn bb(self) -> BitBoard {
        ANTIDIAGONAL_BB[self as usize]
    }
}

impl From<u8> for Diagonal {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { transmute::<u8, Self>(n) }
    }
}

impl From<u8> for AntiDiagonal {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe {
            transmute::<u8, Self>(n)
        }
    }
}
