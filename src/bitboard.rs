use std::fmt;
use std::ops::*;

use super::attacks;
use super::color::*;
use super::file::*;
use super::square::*;
use super::types::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Bitboard(pub Hash);

#[macro_export]
macro_rules! B {
    ($x:expr) => {
        Bitboard($x)
    };
}

impl Bitboard {
    #[inline(always)]
    pub fn lsb(&self) -> SQ {
        SQ::from(self.0.trailing_zeros() as u8)
    }

    #[inline(always)]
    pub fn msb(&self) -> SQ {
        SQ::from((63 - self.0.leading_zeros()) as u8)
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

    pub fn shift(self, dir: Direction) -> Self {
        match dir {
            Direction::North => self << 8,
            Direction::South => self >> 8,
            Direction::NorthNorth => self << 16,
            Direction::SouthSouth => self >> 16,
            Direction::East => (self << 1) & !File::A.bb(),
            Direction::West => (self >> 1) & !File::H.bb(),
            Direction::NorthEast => (self & !File::H.bb()) << 9,
            Direction::NorthWest => (self & !File::A.bb()) << 7,
            Direction::SouthEast => (self & !File::H.bb()) >> 7,
            Direction::SouthWest => (self & !File::A.bb()) >> 9,
        }
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
}

//////////////////////////////////////////////
// Static
//////////////////////////////////////////////

impl Bitboard {
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

impl From<Hash> for Bitboard {
    fn from(value: Hash) -> Self {
        Self(value)
    }
}

//////////////////////////////////////////////
// Shifting Operations
//////////////////////////////////////////////

impl<T> Shl<T> for Bitboard
where
    Hash: Shl<T, Output = Hash>,
{
    type Output = Self;

    #[inline(always)]
    fn shl(self, rhs: T) -> Self::Output {
        Self(self.0 << rhs)
    }
}

impl<T> ShlAssign<T> for Bitboard
where
    Hash: ShlAssign<T>,
{
    #[inline(always)]
    fn shl_assign(&mut self, rhs: T) {
        self.0 <<= rhs;
    }
}

impl<T> Shr<T> for Bitboard
where
    Hash: Shr<T, Output = Hash>,
{
    type Output = Self;

    #[inline(always)]
    fn shr(self, rhs: T) -> Self::Output {
        Self(self.0 >> rhs)
    }
}

impl<T> ShrAssign<T> for Bitboard
where
    Hash: ShrAssign<T>,
{
    #[inline(always)]
    fn shr_assign(&mut self, rhs: T) {
        self.0 >>= rhs;
    }
}

//////////////////////////////////////////////
// Bitwise Operations
//////////////////////////////////////////////

impl BitAnd for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for Bitboard {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Not for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

//////////////////////////////////////////////
// Arithmetic for Magic BitBoards
//////////////////////////////////////////////

impl Mul for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_mul(rhs.0))
    }
}

impl Sub for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

//////////////////////////////////////////////
// Iterator
//////////////////////////////////////////////

impl Iterator for Bitboard {
    type Item = SQ;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            return None;
        }
        Some(self.pop_lsb())
    }
}

//////////////////////////////////////////////
// Display
//////////////////////////////////////////////

impl fmt::Debug for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut result = String::new();
        for i in (0..=56).rev().step_by(8) {
            for j in 0..8 {
                result.push_str(format!("{} ", self.0 >> (i + j) & 1).as_ref());
            }
            result.push('\n');
        }
        write!(f, "{}", result)
    }
}

impl fmt::Display for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

//////////////////////////////////////////////
// Constants
//////////////////////////////////////////////

impl Bitboard {
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

static mut BETWEEN_BB: [[Bitboard; SQ::N_SQUARES]; SQ::N_SQUARES] =
    [[B!(0); SQ::N_SQUARES]; SQ::N_SQUARES];
static mut LINES_BB: [[Bitboard; SQ::N_SQUARES]; SQ::N_SQUARES] =
    [[B!(0); SQ::N_SQUARES]; SQ::N_SQUARES];

fn init_between(between_bb: &mut [[Bitboard; SQ::N_SQUARES]; SQ::N_SQUARES]) {
    for sq1 in Bitboard::ALL {
        for sq2 in Bitboard::ALL {
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

fn init_lines(lines_bb: &mut [[Bitboard; SQ::N_SQUARES]; SQ::N_SQUARES]) {
    for sq1 in Bitboard::ALL {
        for sq2 in Bitboard::ALL {
            if sq1.file() == sq2.file() || sq1.rank() == sq2.rank() {
                lines_bb[sq1.index()][sq2.index()] =
                    attacks::rook_attacks_for_init(sq1, Bitboard::ZERO)
                        & attacks::rook_attacks_for_init(sq2, Bitboard::ZERO)
                        | sq1.bb()
                        | sq2.bb();
            } else if sq1.diagonal() == sq2.diagonal() || sq1.antidiagonal() == sq2.antidiagonal() {
                lines_bb[sq1.index()][sq2.index()] =
                    attacks::bishop_attacks_for_init(sq1, Bitboard::ZERO)
                        & attacks::bishop_attacks_for_init(sq2, Bitboard::ZERO)
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
