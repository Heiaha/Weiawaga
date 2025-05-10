use super::attacks;
use super::piece::*;
use super::square::*;
use super::types::*;
use std::fmt;
use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Mul, Not, Shl, ShlAssign, Shr,
    ShrAssign, Sub,
};
use std::sync::LazyLock;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Bitboard(pub Hash);

#[macro_export]
macro_rules! B {
    ($x:expr) => {
        Bitboard($x)
    };
}

impl Bitboard {
    pub fn lsb(&self) -> SQ {
        SQ::from(self.0.trailing_zeros() as u8)
    }

    pub fn msb(&self) -> SQ {
        SQ::from((63 - self.0.leading_zeros()) as u8)
    }

    pub fn pop_lsb(&mut self) -> SQ {
        let s = self.lsb();
        self.0 &= self.0 - 1;
        s
    }

    pub fn is_several(&self) -> bool {
        self.0 & (self.0.wrapping_sub(1)) != 0
    }

    pub fn is_single(&self) -> bool {
        self.0 != 0 && !self.is_several()
    }

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

    pub fn reverse(self) -> Self {
        Self(self.0.reverse_bits())
    }

    pub fn fill(self, dir: Direction) -> Self {
        match dir {
            Direction::North => self | (self << 8) | (self << 16) | (self << 32),
            Direction::South => self | (self >> 8) | (self >> 16) | (self >> 32),
            _ => {
                panic!("Filling a file by something other than North or South.")
            }
        }
    }
}

//////////////////////////////////////////////
// Static
//////////////////////////////////////////////

impl Bitboard {
    pub fn line(sq1: SQ, sq2: SQ) -> Self {
        LINES_BB[sq1][sq2]
    }

    pub fn between(sq1: SQ, sq2: SQ) -> Self {
        BETWEEN_BB[sq1][sq2]
    }

    pub fn oo_mask(c: Color) -> Self {
        match c {
            Color::White => Self::WHITE_OO_MASK,
            Color::Black => Self::BLACK_OO_MASK,
        }
    }

    pub fn ooo_mask(c: Color) -> Self {
        match c {
            Color::White => Self::WHITE_OOO_MASK,
            Color::Black => Self::BLACK_OOO_MASK,
        }
    }

    pub fn oo_blockers_mask(c: Color) -> Self {
        match c {
            Color::White => Self::WHITE_OO_BLOCKERS_AND_ATTACKERS_MASK,
            Color::Black => Self::BLACK_OO_BLOCKERS_AND_ATTACKERS_MASK,
        }
    }

    pub fn ooo_blockers_mask(c: Color) -> Self {
        match c {
            Color::White => Self::WHITE_OOO_BLOCKERS_AND_ATTACKERS_MASK,
            Color::Black => Self::BLACK_OOO_BLOCKERS_AND_ATTACKERS_MASK,
        }
    }

    pub fn ignore_ooo_danger(c: Color) -> Self {
        match c {
            Color::White => Self::WHITE_OOO_DANGER,
            Color::Black => Self::BLACK_OOO_DANGER,
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

    fn shl(self, rhs: T) -> Self::Output {
        Self(self.0 << rhs)
    }
}

impl<T> ShlAssign<T> for Bitboard
where
    Hash: ShlAssign<T>,
{
    fn shl_assign(&mut self, rhs: T) {
        self.0 <<= rhs;
    }
}

impl<T> Shr<T> for Bitboard
where
    Hash: Shr<T, Output = Hash>,
{
    type Output = Self;

    fn shr(self, rhs: T) -> Self::Output {
        Self(self.0 >> rhs)
    }
}

impl<T> ShrAssign<T> for Bitboard
where
    Hash: ShrAssign<T>,
{
    fn shr_assign(&mut self, rhs: T) {
        self.0 >>= rhs;
    }
}

//////////////////////////////////////////////
// Bitwise Operations
//////////////////////////////////////////////

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for Bitboard {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

//////////////////////////////////////////////
// Arithmetic for Magic BitBoards
//////////////////////////////////////////////

impl Mul for Bitboard {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_mul(rhs.0))
    }
}

impl Sub for Bitboard {
    type Output = Self;

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
                result.push_str(format!("{} ", self.0 >> (i + j) & 1).as_str());
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

static BETWEEN_BB: LazyLock<SQMap<SQMap<Bitboard>>> = LazyLock::new(|| {
    let mut between_bb = SQMap::new([SQMap::new([B!(0); SQ::N_SQUARES]); SQ::N_SQUARES]);
    for sq1 in Bitboard::ALL {
        for sq2 in Bitboard::ALL {
            let sqs = sq1.bb() | sq2.bb();
            if sq1.file() == sq2.file() || sq1.rank() == sq2.rank() {
                between_bb[sq1][sq2] = attacks::rook_attacks_for_init(sq1, sqs)
                    & attacks::rook_attacks_for_init(sq2, sqs);
            } else if sq1.diagonal() == sq2.diagonal() || sq1.antidiagonal() == sq2.antidiagonal() {
                between_bb[sq1][sq2] = attacks::bishop_attacks_for_init(sq1, sqs)
                    & attacks::bishop_attacks_for_init(sq2, sqs);
            }
        }
    }
    between_bb
});
static LINES_BB: LazyLock<SQMap<SQMap<Bitboard>>> = LazyLock::new(|| {
    let mut lines_bb = SQMap::new([SQMap::new([B!(0); SQ::N_SQUARES]); SQ::N_SQUARES]);
    for sq1 in Bitboard::ALL {
        for sq2 in Bitboard::ALL {
            if sq1.file() == sq2.file() || sq1.rank() == sq2.rank() {
                lines_bb[sq1][sq2] = attacks::rook_attacks_for_init(sq1, Bitboard::ZERO)
                    & attacks::rook_attacks_for_init(sq2, Bitboard::ZERO)
                    | sq1.bb()
                    | sq2.bb();
            } else if sq1.diagonal() == sq2.diagonal() || sq1.antidiagonal() == sq2.antidiagonal() {
                lines_bb[sq1][sq2] = attacks::bishop_attacks_for_init(sq1, Bitboard::ZERO)
                    & attacks::bishop_attacks_for_init(sq2, Bitboard::ZERO)
                    | sq1.bb()
                    | sq2.bb();
            }
        }
    }
    lines_bb
});
