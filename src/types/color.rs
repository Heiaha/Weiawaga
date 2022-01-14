use std::mem::transmute;
use std::ops::Not;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    #[inline(always)]
    pub fn index(self) -> usize {
        self as usize
    }
}

impl From<u8> for Color {
    #[inline(always)]
    fn from(n: u8) -> Self {
        unsafe { transmute::<u8, Self>(n) }
    }
}

impl Not for Color {
    type Output = Color;

    #[inline(always)]
    fn not(self) -> Color {
        Color::from((self as u8) ^ 1)
    }
}

impl Color {
    pub const N_COLORS: usize = 2;
}
