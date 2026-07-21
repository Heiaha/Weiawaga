use super::bitboard::*;
use super::piece::*;
use super::square::*;
use super::traits::*;
use super::types::*;
use std::fmt;
use std::ops::BitOr;

pub type CastlingMap<T> = EnumMap<T, { CastlingRights::COUNT }>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct CastlingRights(u8);

impl CastlingRights {
    pub const fn oo(color: Color) -> Self {
        match color {
            Color::White => Self::WHITE_OO,
            Color::Black => Self::BLACK_OO,
        }
    }

    pub const fn ooo(color: Color) -> Self {
        match color {
            Color::White => Self::WHITE_OOO,
            Color::Black => Self::BLACK_OOO,
        }
    }

    // The between-king-and-rook squares that must be empty, and the king's
    // crossing squares that must be unattacked; they differ only for the
    // queenside rook path through the b-file.
    pub fn oo_path(color: Color) -> Bitboard {
        Self::OO_PATH[color]
    }

    pub fn oo_king_path(color: Color) -> Bitboard {
        Self::OO_KING_PATH[color]
    }

    pub fn ooo_path(color: Color) -> Bitboard {
        Self::OOO_PATH[color]
    }

    pub fn ooo_king_path(color: Color) -> Bitboard {
        Self::OOO_KING_PATH[color]
    }

    // The rights a move kills by touching this square: a king square kills
    // both of its side's rights, a rook home square kills its own.
    pub fn killed(sq: SQ) -> Self {
        Self::KILLED[sq]
    }

    pub const fn contains(self, right: Self) -> bool {
        self.0 & right.0 != 0
    }

    pub const fn without(self, rights: Self) -> Self {
        Self(self.0 & !rights.0)
    }

    const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Enumerable for CastlingRights {
    const COUNT: usize = 16;
}

impl BitOr for CastlingRights {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

impl TryFrom<&str> for CastlingRights {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s == "-" {
            return Ok(Self::NONE);
        }
        s.chars().try_fold(Self::NONE, |rights, ch| {
            let right = match ch {
                'K' => Self::WHITE_OO,
                'Q' => Self::WHITE_OOO,
                'k' => Self::BLACK_OO,
                'q' => Self::BLACK_OOO,
                _ => return Err("Invalid castling rights."),
            };
            Ok(rights | right)
        })
    }
}

impl fmt::Display for CastlingRights {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if *self == Self::NONE {
            return write!(f, "-");
        }
        for (right, symbol) in [
            (Self::WHITE_OO, 'K'),
            (Self::WHITE_OOO, 'Q'),
            (Self::BLACK_OO, 'k'),
            (Self::BLACK_OOO, 'q'),
        ] {
            if self.contains(right) {
                write!(f, "{symbol}")?;
            }
        }
        Ok(())
    }
}

impl CastlingRights {
    pub const NONE: Self = Self(0b0000);
    pub const WHITE_OO: Self = Self(0b0001);
    pub const WHITE_OOO: Self = Self(0b0010);
    pub const BLACK_OO: Self = Self(0b0100);
    pub const BLACK_OOO: Self = Self(0b1000);

    const OO_PATH: ColorMap<Bitboard> = ColorMap::new([B!(0x60), B!(0x6000000000000000)]);
    const OO_KING_PATH: ColorMap<Bitboard> = Self::OO_PATH;
    const OOO_PATH: ColorMap<Bitboard> = ColorMap::new([B!(0xe), B!(0xE00000000000000)]);
    const OOO_KING_PATH: ColorMap<Bitboard> = ColorMap::new([B!(0xc), B!(0xC00000000000000)]);

    const KILLED: SQMap<Self> = {
        let mut killed = [Self::NONE; SQ::COUNT];
        killed[SQ::A1 as usize] = Self::WHITE_OOO;
        killed[SQ::E1 as usize] = Self::WHITE_OO.union(Self::WHITE_OOO);
        killed[SQ::H1 as usize] = Self::WHITE_OO;
        killed[SQ::A8 as usize] = Self::BLACK_OOO;
        killed[SQ::E8 as usize] = Self::BLACK_OO.union(Self::BLACK_OOO);
        killed[SQ::H8 as usize] = Self::BLACK_OO;
        SQMap::new(killed)
    };
}
