use crate::types::piece::*;
use std::ops::*;

pub type Value = i32;
pub type Phase = i32;

#[derive(Clone, Copy, Debug, Eq)]
pub struct Score(Value);

macro_rules! S {
    ($x:expr, $y:expr) => {
        Score::new($x, $y)
    };
}

impl Score {
    #[inline(always)]
    pub const fn new(mg: Value, eg: Value) -> Self {
        Self((mg << 16) + eg)
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
        value.abs() >= Self::INF >> 1
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

impl<T> Mul<T> for Score
where
    Value: Mul<T, Output = Value>,
{
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: T) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl<T> MulAssign<T> for Score
where
    Value: MulAssign<T>,
{
    #[inline(always)]
    fn mul_assign(&mut self, rhs: T) {
        self.0 *= rhs;
    }
}

impl PartialEq for Score {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

//////////////////////////////////////////////
// Constants
//////////////////////////////////////////////

impl Score {
    pub const ZERO: Self = Self(0);
    pub const INF: Value = 32000;
    pub const TOTAL_PHASE: Phase = 384;

    const PIECE_PHASES: [Phase; PieceType::N_PIECE_TYPES] = [0, 16, 16, 32, 64, 0];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_mgeg() {
        assert_eq!(S!(1, 1).mg(), 1);
        assert_eq!(S!(1, 1).eg(), 1);
        assert_eq!(S!(1, -1).mg(), 1);
        assert_eq!(S!(1, -1).eg(), -1);
        assert_eq!(S!(-1, 1).mg(), -1);
        assert_eq!(S!(-1, 1).eg(), 1);
        assert_eq!(S!(-1, -1).mg(), -1);
        assert_eq!(S!(-1, -1).eg(), -1);
    }

    #[test]
    fn test_score_calculus() {
        assert_eq!(S!(1, 2) + S!(3, 4), S!(4, 6));
        assert_eq!(S!(-1, -2) + S!(3, 4), S!(2, 2));
        assert_eq!(S!(3, 4) - S!(1, 2), S!(2, 2));
        assert_eq!(S!(3, 0) - S!(1, 2), S!(2, -2));
    }
}
