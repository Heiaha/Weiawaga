use super::score::Score;
use crate::types::bitboard::BitBoard;
use crate::types::color::{Color, N_COLORS};
use crate::types::piece::{Piece, N_PIECES};
use crate::types::square::{Direction, N_SQUARES, SQ};

pub const TEMPO: [Score; 1] = [S!(18, 17)];

const PIECE_TYPE_VALUES: [Score; 6] = [
    S!(109, 114),
    S!(379, 316),
    S!(367, 323),
    S!(496, 536),
    S!(1025, 1022),
    S!(0, 0),
];

//pawn scoring
pub const PAWN_SCORES: [Score; 3] = [S!(-6, 33), S!(-1, -17), S!(-21, 4)];
pub const IX_PASSED_PAWN_VALUE: usize = 0;
pub const IX_DOUBLED_PAWN_PENALTY: usize = 1;
pub const IX_ISOLATED_PAWN_PENALTY: usize = 2;

//bishop scoring
pub const BISHOP_SCORES: [Score; 3] = [S!(-2, -8), S!(30, 11), S!(54, 59)];
pub const IX_BISHOP_SAME_COLOR_PAWN_PENALTY: usize = 0; // per pawn
pub const IX_BISHOP_ATTACKS_CENTER: usize = 1;
pub const IX_BISHOP_PAIR_VALUE: usize = 2;

//rook scoring
pub const ROOK_SCORES: [Score; 3] = [S!(-26, -36), S!(50, 22), S!(16, 28)];
pub const IX_KING_TRAPPING_ROOK_PENALTY: usize = 0;
pub const IX_ROOK_ON_OPEN_FILE: usize = 1;
pub const IX_ROOK_ON_SEMIOPEN_FILE: usize = 2;

//king scoring
pub const KING_SCORES: [Score; 1] = [S!(18, -6)];
pub const IX_KING_PAWN_SHIELD_BONUS: usize = 0;

const PIECE_TYPE_TABLES: [[Score; N_SQUARES]; 6] = [
    [
        S!(-57, -46), S!(-57, -46), S!(-57, -46), S!(-57, -46), S!(-57, -46), S!(-57, -46), S!(-57, -46), S!(-57, -46),
        S!(-36, -25), S!(-27, -37), S!(-37, -26), S!(-56, -21), S!(-50, -21), S!( -1, -32), S!(  2, -42), S!(-28, -44),
        S!(-24, -38), S!(-30, -36), S!(-14, -44), S!(-26, -32), S!(-12, -36), S!( -6, -42), S!(  5, -51), S!(-12, -49),
        S!(-26, -28), S!(-26, -32), S!(-17, -40), S!(  1, -40), S!(  4, -40), S!( -1, -44), S!(-22, -38), S!(-29, -38),
        S!(-13,  -6), S!( -3, -16), S!( -1, -24), S!( 12, -35), S!( 12, -35), S!(  6, -31), S!( -6, -19), S!(-30, -19),
        S!(  7,  51), S!(  2,  51), S!( 10,  43), S!( 16,  33), S!( 20,  23), S!( 22,  20), S!(  4,  42), S!(-10,  42),
        S!( 52, 116), S!( 52, 116), S!( 40,  96), S!( 40,  92), S!( 38,  92), S!( 44,  92), S!( 38,  98), S!( 38, 116),
        S!(-46, -54), S!(-46, -54), S!(-46, -54), S!(-46, -54), S!(-46, -54), S!(-46, -54), S!(-46, -54), S!(-46, -54),
    ],
    [
        S!(-93, -66), S!(-47, -81), S!(-66, -61), S!(-52, -55), S!(-36, -57), S!(-46, -59), S!(-40, -79), S!(-60, -93),
        S!(-59, -75), S!(-57, -59), S!(-30, -39), S!(-24, -36), S!(-19, -32), S!( -2, -47), S!(-32, -51), S!(-30, -79),
        S!(-48, -57), S!(-25, -34), S!( -6, -33), S!( -4, -18), S!(  6, -22), S!(  2, -34), S!(  3, -41), S!(-36, -55),
        S!(-40, -51), S!(-16, -37), S!(  1, -16), S!(  3,  -9), S!( 17, -17), S!(  8, -17), S!(  2, -26), S!(-28, -53),
        S!(-32, -51), S!( -3, -28), S!(  9, -16), S!( 34, -11), S!( 26, -11), S!( 26, -10), S!(  2, -25), S!(-22, -44),
        S!(-53, -64), S!(  5, -38), S!( 13, -18), S!( 22, -17), S!( 22, -23), S!( 17, -24), S!(  7, -40), S!(-23, -68),
        S!(-67, -70), S!(-47, -50), S!(  7, -40), S!(  1, -32), S!(-15, -40), S!(  7, -40), S!(-24, -56), S!(-48, -80),
        S!(-78, -92), S!(-66, -76), S!(-56, -48), S!(-46, -66), S!(-30, -52), S!(-58, -72), S!(-60, -82), S!(-78, -92),
    ],
    [
        S!(-44, -40), S!(-14, -17), S!(-31, -25), S!(-29, -13), S!(-23, -17), S!(-26, -22), S!(-32, -26), S!(-40, -46),
        S!( -6, -30), S!( -3, -21), S!(  6, -12), S!(-13,  -2), S!( -5,   0), S!(  5, -12), S!( 19, -21), S!(-17, -34),
        S!(-14, -18), S!( -4,  -4), S!(  0, -12), S!(  2,   8), S!(  2,  12), S!(  1, -16), S!(  5,  -7), S!( -5, -17),
        S!(-25, -15), S!( -3,  -1), S!(  1,   9), S!( 17,  11), S!( 19,   3), S!( -1,   5), S!( -5,  -9), S!(-11, -19),
        S!(-15, -11), S!(-14,   6), S!(  5,   4), S!( 16,   9), S!( 16,   9), S!( 11,   6), S!( -5,  -2), S!(-14, -11),
        S!(-30, -11), S!(  4,  -5), S!( -5, -18), S!(  8,  -1), S!(  6,  -7), S!( -1, -16), S!(  0,  -1), S!(-16,  -9),
        S!(-30, -19), S!(-20, -23), S!(-18,  -3), S!(-20, -21), S!( -2,  -9), S!(  0,  -9), S!(-14, -23), S!(-30, -31),
        S!(-40, -43), S!(-22, -21), S!(-30, -29), S!(-26, -21), S!(-24, -19), S!(-28, -23), S!(-18, -21), S!(-40, -43),
    ],
    [
        S!(-27, -17), S!(-31, -11), S!(-25, -17), S!(-17, -22), S!(-15, -26), S!(-11, -15), S!(-41, -15), S!(-15, -29),
        S!(-48, -23), S!(-37, -24), S!(-43, -22), S!(-41, -20), S!(-35, -26), S!(-15, -28), S!(-35, -26), S!(-48, -21),
        S!(-48, -23), S!(-35, -21), S!(-37, -27), S!(-41, -25), S!(-32, -29), S!(-26, -24), S!(-26, -24), S!(-43, -25),
        S!(-47, -14), S!(-38, -15), S!(-34, -15), S!(-29, -21), S!(-24, -26), S!(-30, -20), S!(-16, -22), S!(-37, -23),
        S!(-33, -13), S!(-32, -14), S!(-14, -13), S!(-14, -22), S!(-13, -22), S!( -7, -11), S!(-27, -17), S!(-24, -12),
        S!(-23,  -9), S!(-12, -11), S!(-11, -15), S!( -9, -17), S!(-15, -24), S!( -2, -12), S!(  0,  -8), S!(-17, -16),
        S!( -3,  -4), S!(  0,  -5), S!(  6,  -9), S!(  6, -12), S!( 10, -19), S!( 16,  -4), S!( -4,  -4), S!(  7,  -9),
        S!(  2, -10), S!(  0, -11), S!( -5, -13), S!(  3, -18), S!(  3, -15), S!(  1, -13), S!( -7, -13), S!( -5, -14),
    ],
    [
        S!(-22, -27), S!(-24, -28), S!(-16, -27), S!(  3, -25), S!(-20, -21), S!(-25, -32), S!(-25, -32), S!(-35, -42),
        S!(-25, -32), S!(-15, -22), S!(  5, -22), S!( -5, -18), S!(  3, -18), S!(  1, -22), S!(-15, -22), S!(-13, -26),
        S!(-25, -22), S!( -8, -17), S!( -8,  -3), S!( -6, -10), S!( -6,  -9), S!( -4,   2), S!(  3,  -5), S!( -7, -15),
        S!(-15, -19), S!(-15,  -7), S!(-10,  -4), S!( -8,  10), S!(  4,   8), S!(  0,   8), S!(  1,   3), S!(-10,  -5),
        S!(-20, -19), S!(-15, -14), S!(-10,  -5), S!( -6,   5), S!( 14,   7), S!( 14,   5), S!(  7,   0), S!(  0,  -5),
        S!(-25, -26), S!(-15, -18), S!(  0,  -9), S!(  8,   5), S!( 16,   5), S!( 16,   5), S!( 11,   0), S!(  1, -10),
        S!(-25, -28), S!(-15, -18), S!( -5,  -2), S!(  5,   0), S!(  1,   0), S!( 11,   0), S!( 11,   0), S!(  1, -10),
        S!(-35, -34), S!( -7, -12), S!( -3, -10), S!( -2,  -5), S!(  2,  -5), S!( -3, -10), S!( -3, -10), S!(-13, -20),
    ],
    [
        S!(  5, -71), S!( 19, -51), S!( -2, -41), S!(-16, -42), S!( 14, -43), S!( -9, -39), S!( 25, -51), S!( 19, -77),
        S!(  6, -51), S!( 15, -34), S!(-19, -12), S!(-21, -14), S!(-21, -10), S!(-21, -10), S!( 21, -28), S!( 17, -44),
        S!(-17, -42), S!(-29, -16), S!(-39,  -4), S!(-41,   4), S!(-41,   6), S!(-41,   2), S!(-13, -10), S!(-23, -30),
        S!(-35, -44), S!(-31, -20), S!(-51,   6), S!(-61,  14), S!(-61,  14), S!(-49,   7), S!(-35,  -7), S!(-39, -33),
        S!(-31, -29), S!(-33,  -1), S!(-47,   9), S!(-69,  15), S!(-69,  15), S!(-43,  15), S!(-31,   3), S!(-27, -21),
        S!(-23, -17), S!(-33,   3), S!(-39,  11), S!(-67,   5), S!(-65,   5), S!(-35,  29), S!(-33,   3), S!(-23, -17),
        S!(-27, -21), S!(-33,  -7), S!(-35,  -3), S!(-49,   5), S!(-51,   1), S!(-39,   3), S!(-39,  -7), S!(-33, -17),
        S!(-45, -73), S!(-41, -39), S!(-41, -31), S!(-61, -30), S!(-67, -28), S!(-49, -18), S!(-47, -28), S!(-37, -38),
    ],
];

pub static mut PIECE_VALUES: [Score; N_PIECES] = [Score::ZERO; N_PIECES];
pub static mut PIECE_TABLES: [[Score; N_SQUARES]; N_PIECES] = [[Score::ZERO; N_SQUARES]; N_PIECES];
pub static mut PAWN_SHIELD_MASKS: [[BitBoard; N_SQUARES]; N_COLORS] = [[BitBoard::ZERO; N_SQUARES]; N_COLORS];

#[inline(always)]
pub fn piece_value(pc: Piece) -> Score {
    unsafe { PIECE_VALUES[pc.index()] }
}

#[inline(always)]
pub fn piece_sq_value(pc: Piece, sq: SQ) -> Score {
    unsafe { PIECE_TABLES[pc.index()][sq.index()] }
}

fn init_pawn_shields(pawn_shields: &mut [[BitBoard; N_SQUARES]; N_COLORS]) {
    for sq in SQ::A1..=SQ::H8 {
        pawn_shields[Color::White.index()][sq.index()] = sq.bb().shift(Direction::North, 1)
            | sq.bb().shift(Direction::NorthEast, 1)
            | sq.bb().shift(Direction::NorthWest, 1);
        pawn_shields[Color::Black.index()][sq.index()] = sq.bb().shift(Direction::South, 1)
            | sq.bb().shift(Direction::SouthEast, 1)
            | sq.bb().shift(Direction::SouthWest, 1);
    }
}

fn init_piece_values(
    piece_values: &mut [Score; N_PIECES],
    piece_tables: &mut [[Score; N_SQUARES]; N_PIECES],
) {
    for pc in Piece::WhitePawn..=Piece::WhiteKing {
        piece_values[pc.index()] = PIECE_TYPE_VALUES[pc.type_of().index()];
        piece_values[pc.flip().index()] = -PIECE_TYPE_VALUES[pc.type_of().index()];
        for sq in SQ::A1..=SQ::H8 {
            piece_tables[pc.index()][sq.index()] = PIECE_TYPE_TABLES[pc.index()][sq.index()];
            piece_tables[pc.flip().index()][sq.index()] =
                -PIECE_TYPE_TABLES[pc.index()][sq.square_mirror().index()];
        }
    }
}

pub fn init_eval() {
    unsafe {
        init_piece_values(&mut PIECE_VALUES, &mut PIECE_TABLES);
        init_pawn_shields(&mut PAWN_SHIELD_MASKS);
    }
}
