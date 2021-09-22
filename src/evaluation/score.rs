use crate::types::piece::{PieceType, N_PIECE_TYPES};
use std::fmt;
use std::ops::*;

pub type Value = i32;
pub type Phase = i32;

#[derive(Clone, Copy, Debug)]
pub struct Score(i32);

macro_rules! S {
    ($x:expr, $y:expr) => {
        Score::new($x, $y)
    };
}

impl Score {
    #[inline(always)]
    pub const fn new(mg: i32, eg: i32) -> Self {
        Score((mg << 16) + eg)
    }

    #[inline(always)]
    pub fn mg(&self) -> Value {
        ((self.0 + 0x8000) >> 16) as Value
    }

    #[inline(always)]
    pub fn eg(&self) -> Value {
        self.0 as i16 as Value
    }

    #[inline(always)]
    pub fn eval(&self, phase: Phase) -> Value {
        (self.mg() * (Self::TOTAL_PHASE - phase) + self.eg() * phase) / Self::TOTAL_PHASE
    }

    #[inline(always)]
    pub fn piece_phase(pt: PieceType) -> Phase {
        Self::PIECE_PHASES[pt.index()]
    }

    #[inline(always)]
    pub fn is_checkmate(value: Value) -> bool {
        2 * value.abs() >= Score::INF
    }

    pub fn scores(&self) -> (Value, Value) {
        (self.mg(), self.eg())
    }
}

//////////////////////////////////////////////
// Arithmetic
//////////////////////////////////////////////

impl Neg for Score {
    type Output = Self;

    #[inline(always)]
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl Add for Score {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Score {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0 + rhs.0;
    }
}

impl Sub for Score {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Score {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0 - rhs.0;
    }
}

impl Mul<Value> for Score {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: Value) -> Self::Output {
        Self(self.0.wrapping_mul(rhs))
    }
}

impl MulAssign<Value> for Score {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: i32) {
        self.0 = self.0.wrapping_mul(rhs);
    }
}

//////////////////////////////////////////////
// Display
//////////////////////////////////////////////

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Score(mg: {} eg: {})", self.mg(), self.eg())
    }
}

//////////////////////////////////////////////
// Constants
//////////////////////////////////////////////

impl Score {
    pub const ZERO: Self = Score(0);
    pub const INF: Value = 200000;
    pub const TOTAL_PHASE: Phase = 384;

    const PIECE_PHASES: [Phase; N_PIECE_TYPES] = [0, 16, 16, 32, 64, 0];
}
