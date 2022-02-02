use super::attacks;
use super::color::*;
use super::file::*;
use super::square::*;
use crate::evaluation::score::*;
use std::fmt;
use std::fmt::Formatter;
use std::ops::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BitBoard(pub u64);

pub type Hash = BitBoard;

macro_rules! B {
    ($x:expr) => {
        BitBoard($x)
    };
}

impl BitBoard {
    #[inline(always)]
    pub fn lsb(&self) -> SQ {
        SQ::from(self.0.trailing_zeros() as u8)
    }

    #[inline(always)]
    pub fn msb(&self) -> SQ {
        SQ::from(63_u8.wrapping_sub(self.0.leading_zeros() as u8))
    }

    #[inline(always)]
    pub fn pop_lsb(&mut self) -> SQ {
        let s = self.lsb();
        self.0 &= self.0 - 1;
        s
    }

    #[inline(always)]
    pub fn is_several(&self) -> bool {
        self.0 & (self.0.wrapping_sub(1)) != 0
    }

    #[inline(always)]
    pub fn is_single(&self) -> bool {
        self.0 != 0 && !self.is_several()
    }

    #[inline(always)]
    pub fn pop_count(&self) -> Value {
        self.0.count_ones() as Value
    }

    pub fn shift(self, dir: Direction, n: u32) -> Self {
        let mut result = self;

        match dir {
            Direction::North => {
                for _ in 0..n {
                    result <<= 8;
                }
            }
            Direction::South => {
                for _ in 0..n {
                    result >>= 8;
                }
            }
            Direction::East => {
                for _ in 0..n {
                    result = (result << 1) & !File::A.bb();
                }
            }
            Direction::West => {
                for _ in 0..n {
                    result = (result >> 1) & !File::H.bb();
                }
            }
            Direction::NorthEast => {
                for _ in 0..n {
                    result = (result & !File::H.bb()) << 9;
                }
            }
            Direction::NorthWest => {
                for _ in 0..n {
                    result = (result & !File::A.bb()) << 7;
                }
            }
            Direction::SouthEast => {
                for _ in 0..n {
                    result = (result & !File::H.bb()) >> 7;
                }
            }
            Direction::SouthWest => {
                for _ in 0..n {
                    result = (result & !File::A.bb()) >> 9;
                }
            }
            _ => {}
        }
        result
    }

    #[inline(always)]
    pub fn reverse(self) -> Self {
        Self(self.0.reverse_bits())
    }

    pub fn fill(self, dir: Direction) -> Self {
        let mut result = self;
        match dir {
            Direction::North => {
                result |= result << 8;
                result |= result << 16;
                result |= result << 32;
            }
            Direction::South => {
                result |= result >> 8;
                result |= result >> 16;
                result |= result >> 32;
            }
            _ => {
                panic!("Filling a file by something other than North or South.")
            }
        }
        result
    }

    #[inline(always)]
    pub fn file_fill(self) -> Self {
        self.fill(Direction::North) | self.fill(Direction::South)
    }
}

//////////////////////////////////////////////
// Static
//////////////////////////////////////////////

impl BitBoard {
    #[inline(always)]
    pub fn line(sq1: SQ, sq2: SQ) -> Self {
        unsafe { LINES_BB[sq1.index()][sq2.index()] }
    }
    #[inline(always)]
    pub fn between(sq1: SQ, sq2: SQ) -> Self {
        unsafe { BETWEEN_BB[sq1.index()][sq2.index()] }
    }

    #[inline(always)]
    pub fn oo_mask(c: Color) -> Self {
        if c == Color::White {
            Self::WHITE_OO_MASK
        } else {
            Self::BLACK_OO_MASK
        }
    }

    #[inline(always)]
    pub fn ooo_mask(c: Color) -> Self {
        if c == Color::White {
            Self::WHITE_OOO_MASK
        } else {
            Self::BLACK_OOO_MASK
        }
    }

    #[inline(always)]
    pub fn oo_blockers_mask(c: Color) -> Self {
        if c == Color::White {
            Self::WHITE_OO_BLOCKERS_AND_ATTACKERS_MASK
        } else {
            Self::BLACK_OO_BLOCKERS_AND_ATTACKERS_MASK
        }
    }

    #[inline(always)]
    pub fn ooo_blockers_mask(c: Color) -> Self {
        if c == Color::White {
            Self::WHITE_OOO_BLOCKERS_AND_ATTACKERS_MASK
        } else {
            Self::BLACK_OOO_BLOCKERS_AND_ATTACKERS_MASK
        }
    }

    #[inline(always)]
    pub fn ignore_ooo_danger(c: Color) -> Self {
        if c == Color::White {
            Self::WHITE_OOO_DANGER
        } else {
            Self::BLACK_OOO_DANGER
        }
    }
}

impl From<u64> for BitBoard {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

//////////////////////////////////////////////
// Shifting Operations
//////////////////////////////////////////////

impl Shl<u32> for BitBoard {
    type Output = Self;

    #[inline(always)]
    fn shl(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_shl(rhs))
    }
}

impl ShlAssign<u32> for BitBoard {
    #[inline(always)]
    fn shl_assign(&mut self, rhs: u32) {
        self.0 = self.0.wrapping_shl(rhs);
    }
}

impl Shr<u32> for BitBoard {
    type Output = Self;

    #[inline(always)]
    fn shr(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_shr(rhs))
    }
}

impl ShrAssign<u32> for BitBoard {
    #[inline(always)]
    fn shr_assign(&mut self, rhs: u32) {
        self.0 = self.0.wrapping_shr(rhs);
    }
}

//////////////////////////////////////////////
// Bitwise Operations
//////////////////////////////////////////////

impl BitAnd for BitBoard {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for BitBoard {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for BitBoard {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for BitBoard {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor for BitBoard {
    type Output = Self;

    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for BitBoard {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Not for BitBoard {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

//////////////////////////////////////////////
// Arithmetic for Magic BitBoards
//////////////////////////////////////////////

impl Mul for BitBoard {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_mul(rhs.0))
    }
}

impl MulAssign for BitBoard {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_mul(rhs.0);
    }
}

impl Sub for BitBoard {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

//////////////////////////////////////////////
// Iterator
//////////////////////////////////////////////

impl Iterator for BitBoard {
    type Item = SQ;

    fn next(&mut self) -> Option<SQ> {
        if *self == Self::ZERO {
            return None;
        }
        Some(self.pop_lsb())
    }
}

//////////////////////////////////////////////
// Display
//////////////////////////////////////////////

impl fmt::Debug for BitBoard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut result = String::new();
        for i in (0..=56).rev().step_by(8) {
            for j in 0..8 {
                result.push_str(&*format!("{} ", self.0 >> (i + j) & 1));
            }
            result.push('\n');
        }
        write!(f, "{}", result)
    }
}

impl fmt::Display for BitBoard {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

//////////////////////////////////////////////
// Constants
//////////////////////////////////////////////

impl BitBoard {
    pub const ALL: Self = B!(0xffffffffffffffff);
    pub const ZERO: Self = B!(0);
    pub const ONE: Self = B!(1);
    pub const TWO: Self = B!(2);
    pub const LIGHT_SQUARES: Self = B!(0x55AA55AA55AA55AA);
    pub const DARK_SQUARES: Self = B!(0xAA55AA55AA55AA55);

    pub const WHITE_OO_MASK: Self = B!(0x90);
    pub const WHITE_OOO_MASK: Self = B!(0x11);
    pub const WHITE_OO_BLOCKERS_AND_ATTACKERS_MASK: Self = B!(0x60);
    pub const WHITE_OOO_BLOCKERS_AND_ATTACKERS_MASK: Self = B!(0xe);

    pub const BLACK_OO_MASK: Self = B!(0x9000000000000000);
    pub const BLACK_OOO_MASK: Self = B!(0x1100000000000000);
    pub const BLACK_OO_BLOCKERS_AND_ATTACKERS_MASK: Self = B!(0x6000000000000000);
    pub const BLACK_OOO_BLOCKERS_AND_ATTACKERS_MASK: Self = B!(0xE00000000000000);

    pub const ALL_CASTLING_MASK: Self = B!(0x9100000000000091);

    pub const WHITE_OOO_DANGER: Self = B!(0x2);
    pub const BLACK_OOO_DANGER: Self = B!(0x200000000000000);

    pub const CENTER: Self = B!(0x1818000000);
}

static mut BETWEEN_BB: [[BitBoard; SQ::N_SQUARES]; SQ::N_SQUARES] =
    [[B!(0); SQ::N_SQUARES]; SQ::N_SQUARES];
static mut LINES_BB: [[BitBoard; SQ::N_SQUARES]; SQ::N_SQUARES] =
    [[B!(0); SQ::N_SQUARES]; SQ::N_SQUARES];

fn init_between(between_bb: &mut [[BitBoard; SQ::N_SQUARES]; SQ::N_SQUARES]) {
    for sq1 in BitBoard::ALL {
        for sq2 in BitBoard::ALL {
            let sqs = sq1.bb() | sq2.bb();
            if sq1.file() == sq2.file() || sq1.rank() == sq2.rank() {
                between_bb[sq1.index()][sq2.index()] = attacks::rook_attacks_for_init(sq1, sqs)
                    & attacks::rook_attacks_for_init(sq2, sqs);
            } else if sq1.diagonal() == sq2.diagonal() || sq1.antidiagonal() == sq2.antidiagonal() {
                between_bb[sq1.index()][sq2.index()] = attacks::bishop_attacks_for_init(sq1, sqs)
                    & attacks::bishop_attacks_for_init(sq2, sqs);
            }
        }
    }
}

fn init_lines(lines_bb: &mut [[BitBoard; SQ::N_SQUARES]; SQ::N_SQUARES]) {
    for sq1 in BitBoard::ALL {
        for sq2 in BitBoard::ALL {
            if sq1.file() == sq2.file() || sq1.rank() == sq2.rank() {
                lines_bb[sq1.index()][sq2.index()] =
                    attacks::rook_attacks_for_init(sq1, BitBoard::ZERO)
                        & attacks::rook_attacks_for_init(sq2, BitBoard::ZERO)
                        | sq1.bb()
                        | sq2.bb();
            } else if sq1.diagonal() == sq2.diagonal() || sq1.antidiagonal() == sq2.antidiagonal() {
                lines_bb[sq1.index()][sq2.index()] =
                    attacks::bishop_attacks_for_init(sq1, BitBoard::ZERO)
                        & attacks::bishop_attacks_for_init(sq2, BitBoard::ZERO)
                        | sq1.bb()
                        | sq2.bb();
            }
        }
    }
}

pub fn init_bb() {
    unsafe {
        init_between(&mut BETWEEN_BB);
        init_lines(&mut LINES_BB);
    }
}
