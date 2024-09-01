use std::fmt;
use std::ops::Not;

use super::types::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn factor(&self) -> Value {
        match *self {
            Self::White => 1,
            Self::Black => -1,
        }
    }
}

impl From<u8> for Color {
    fn from(n: u8) -> Self {
        unsafe { std::mem::transmute::<u8, Self>(n) }
    }
}

impl Not for Color {
    type Output = Color;

    fn not(self) -> Self {
        Color::from((self as u8) ^ 1)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::White => "w",
                Self::Black => "b",
            }
        )
    }
}

impl TryFrom<char> for Color {
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'w' => Ok(Self::White),
            'b' => Ok(Self::Black),
            _ => Err("Color must be either 'w' or 'b'."),
        }
    }
}

impl Color {
    pub const N_COLORS: usize = 2;
}
