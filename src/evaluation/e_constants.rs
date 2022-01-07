use super::score::Score;
use crate::types::bitboard::*;
use crate::types::color::*;
use crate::types::piece::*;
use crate::types::square::*;

#[rustfmt::skip]
pub static mut TEMPO: [Score; 1] = [S!(  17,   16), ];

#[rustfmt::skip]
pub static mut PIECE_TYPE_VALUES: [Score; 6] = [
    S!( 102,  117), S!( 382,  319), S!( 370,  324), S!( 499,  539), S!(1028, 1025), S!(  0,   0),
];

//pawn scoring
#[rustfmt::skip]
pub static mut PAWN_SCORES: [Score; 3] = [S!(  -7,   32), S!(   2,  -16), S!( -20,    5), ];
pub const IX_PASSED_PAWN_VALUE: usize = 0;
pub const IX_DOUBLED_PAWN_PENALTY: usize = 1;
pub const IX_ISOLATED_PAWN_PENALTY: usize = 2;

//bishop scoring
#[rustfmt::skip]
pub static mut BISHOP_SCORES: [Score; 3] = [S!(   1,   -8), S!(  29,   13), S!(  51,   57), ];
pub const IX_BISHOP_SAME_COLOR_PAWN_PENALTY: usize = 0; // per pawn
pub const IX_BISHOP_ATTACKS_CENTER: usize = 1;
pub const IX_BISHOP_PAIR_VALUE: usize = 2;

//rook scoring
#[rustfmt::skip]
pub static mut ROOK_SCORES: [Score; 3] = [S!( -24,  -36), S!(  52,   20), S!(  18,   27), ];
pub const IX_KING_TRAPPING_ROOK_PENALTY: usize = 0;
pub const IX_ROOK_ON_OPEN_FILE: usize = 1;
pub const IX_ROOK_ON_SEMIOPEN_FILE: usize = 2;

//king scoring
#[rustfmt::skip]
pub static mut KING_SCORES: [Score; 1] = [S!(  21,   -7), ];
pub const IX_KING_PAWN_SHIELD_BONUS: usize = 0;

#[rustfmt::skip]
pub static mut PIECE_TYPE_TABLES: [[Score; N_SQUARES]; 6] = [
    [
        S!( -60,  -49), S!( -60,  -49), S!( -60,  -49), S!( -60,  -49), S!( -60,  -49), S!( -60,  -49), S!( -60,  -49), S!( -60,  -49),
        S!( -35,  -24), S!( -24,  -34), S!( -38,  -29), S!( -53,  -22), S!( -47,  -20), S!(   2,  -31), S!(   5,  -41), S!( -25,  -43),
        S!( -27,  -39), S!( -29,  -33), S!( -17,  -45), S!( -23,  -29), S!(  -9,  -33), S!(  -9,  -39), S!(   8,  -48), S!( -11,  -48),
        S!( -29,  -29), S!( -29,  -33), S!( -20,  -39), S!(  -2,  -43), S!(   1,  -43), S!(  -4,  -47), S!( -25,  -35), S!( -30,  -39),
        S!( -16,   -7), S!(  -6,  -19), S!(  -4,  -23), S!(   9,  -38), S!(   9,  -38), S!(   5,  -32), S!(  -9,  -22), S!( -33,  -20),
        S!(  10,   54), S!(   5,   54), S!(  13,   46), S!(  13,   32), S!(  23,   26), S!(  21,   18), S!(   1,   44), S!( -13,   44),
        S!(  55,  118), S!(  55,  118), S!(  43,   98), S!(  43,   94), S!(  41,   94), S!(  47,   94), S!(  41,  100), S!(  39,  118),
        S!( -49,  -55), S!( -49,  -55), S!( -49,  -55), S!( -49,  -55), S!( -49,  -55), S!( -49,  -55), S!( -49,  -55), S!( -49,  -55),
    ],
    [
        S!( -95,  -66), S!( -45,  -81), S!( -64,  -61), S!( -52,  -55), S!( -36,  -57), S!( -46,  -59), S!( -38,  -79), S!( -60,  -93),
        S!( -59,  -75), S!( -57,  -59), S!( -28,  -39), S!( -22,  -36), S!( -17,  -32), S!(  -4,  -47), S!( -30,  -51), S!( -28,  -79),
        S!( -48,  -57), S!( -23,  -34), S!(  -6,  -33), S!(  -6,  -18), S!(   6,  -22), S!(   0,  -34), S!(   5,  -41), S!( -38,  -55),
        S!( -38,  -51), S!( -14,  -37), S!(  -1,  -16), S!(   4,   -9), S!(  18,  -17), S!(   9,  -17), S!(   3,  -26), S!( -28,  -53),
        S!( -32,  -51), S!(  -3,  -28), S!(   9,  -16), S!(  34,  -11), S!(  26,  -11), S!(  26,  -10), S!(   2,  -25), S!( -22,  -44),
        S!( -53,  -64), S!(   5,  -38), S!(  13,  -18), S!(  22,  -17), S!(  22,  -23), S!(  17,  -24), S!(   7,  -40), S!( -23,  -68),
        S!( -67,  -70), S!( -47,  -50), S!(   7,  -40), S!(   1,  -32), S!( -15,  -40), S!(   7,  -40), S!( -24,  -56), S!( -48,  -80),
        S!( -78,  -92), S!( -66,  -76), S!( -56,  -48), S!( -46,  -66), S!( -30,  -52), S!( -58,  -72), S!( -60,  -82), S!( -78,  -92),
    ],
    [
        S!( -46,  -42), S!( -16,  -19), S!( -29,  -23), S!( -27,  -13), S!( -23,  -15), S!( -24,  -20), S!( -34,  -26), S!( -42,  -44),
        S!(  -6,  -28), S!(  -5,  -23), S!(   6,  -14), S!( -11,   -2), S!(  -7,   -2), S!(   3,  -12), S!(  17,  -23), S!( -15,  -34),
        S!( -16,  -18), S!(  -2,   -4), S!(   0,  -12), S!(   0,    6), S!(   0,   10), S!(   1,  -18), S!(   3,   -7), S!(  -7,  -19),
        S!( -27,  -17), S!(  -1,    1), S!(  -1,   10), S!(  15,   10), S!(  17,    2), S!(  -3,    4), S!(  -5,  -10), S!( -11,  -18),
        S!( -13,  -11), S!( -12,    6), S!(   7,    4), S!(  18,    9), S!(  18,    9), S!(  13,    6), S!(  -7,   -2), S!( -12,  -11),
        S!( -32,  -11), S!(   6,   -5), S!(  -5,  -18), S!(   6,   -1), S!(   6,   -7), S!(  -1,  -16), S!(  -2,   -1), S!( -18,   -9),
        S!( -32,  -19), S!( -22,  -23), S!( -20,   -3), S!( -20,  -21), S!(  -2,   -9), S!(   0,   -9), S!( -14,  -23), S!( -32,  -31),
        S!( -42,  -43), S!( -24,  -21), S!( -32,  -29), S!( -28,  -21), S!( -26,  -19), S!( -30,  -23), S!( -20,  -21), S!( -42,  -43),
    ],
    [
        S!( -27,  -16), S!( -34,  -10), S!( -28,  -16), S!( -20,  -23), S!( -18,  -27), S!( -14,  -16), S!( -44,  -16), S!( -14,  -30),
        S!( -51,  -24), S!( -40,  -23), S!( -46,  -21), S!( -40,  -21), S!( -34,  -27), S!( -14,  -29), S!( -38,  -27), S!( -51,  -22),
        S!( -51,  -22), S!( -38,  -20), S!( -38,  -26), S!( -44,  -24), S!( -35,  -28), S!( -27,  -25), S!( -29,  -24), S!( -46,  -25),
        S!( -48,  -14), S!( -41,  -15), S!( -37,  -15), S!( -30,  -21), S!( -25,  -26), S!( -33,  -20), S!( -15,  -22), S!( -40,  -23),
        S!( -34,  -13), S!( -31,  -14), S!( -13,  -13), S!( -15,  -22), S!( -12,  -22), S!(  -6,  -11), S!( -26,  -17), S!( -27,  -12),
        S!( -26,   -9), S!( -13,  -11), S!( -10,  -15), S!(  -9,  -17), S!( -15,  -24), S!(  -4,  -12), S!(  -2,   -8), S!( -17,  -16),
        S!(  -4,   -4), S!(  -1,   -5), S!(   5,   -9), S!(   5,  -12), S!(   9,  -19), S!(  15,   -4), S!(  -5,   -4), S!(   6,   -9),
        S!(   1,  -10), S!(  -1,  -11), S!(  -6,  -13), S!(   2,  -18), S!(   2,  -15), S!(   0,  -13), S!(  -8,  -13), S!(  -6,  -14),
    ],
    [
        S!( -22,  -27), S!( -24,  -28), S!( -16,  -27), S!(   3,  -25), S!( -20,  -21), S!( -25,  -32), S!( -25,  -32), S!( -35,  -42),
        S!( -25,  -32), S!( -15,  -22), S!(   5,  -22), S!(  -5,  -18), S!(   3,  -18), S!(   1,  -22), S!( -15,  -22), S!( -13,  -26),
        S!( -25,  -22), S!(  -8,  -17), S!(  -8,   -3), S!(  -6,  -10), S!(  -6,   -9), S!(  -4,    2), S!(   3,   -5), S!(  -7,  -15),
        S!( -15,  -19), S!( -15,   -7), S!( -10,   -4), S!(  -8,   10), S!(   4,    8), S!(   0,    8), S!(   1,    3), S!( -10,   -5),
        S!( -20,  -19), S!( -15,  -14), S!( -10,   -5), S!(  -6,    5), S!(  14,    7), S!(  14,    5), S!(   7,    0), S!(   0,   -5),
        S!( -25,  -26), S!( -15,  -18), S!(   0,   -9), S!(   8,    5), S!(  16,    5), S!(  16,    5), S!(  11,    0), S!(   1,  -10),
        S!( -25,  -28), S!( -15,  -18), S!(  -5,   -2), S!(   5,    0), S!(   1,    0), S!(  11,    0), S!(  11,    0), S!(   1,  -10),
        S!( -35,  -34), S!(  -7,  -12), S!(  -3,  -10), S!(  -2,   -5), S!(   2,   -5), S!(  -3,  -10), S!(  -3,  -10), S!( -13,  -20),
    ],
    [
        S!(   5,  -71), S!(  19,  -51), S!(  -2,  -41), S!( -16,  -42), S!(  14,  -43), S!(  -9,  -39), S!(  25,  -51), S!(  19,  -77),
        S!(   6,  -51), S!(  15,  -34), S!( -19,  -12), S!( -21,  -14), S!( -21,  -10), S!( -21,  -10), S!(  21,  -28), S!(  17,  -44),
        S!( -17,  -42), S!( -29,  -16), S!( -39,   -4), S!( -41,    4), S!( -41,    6), S!( -41,    2), S!( -13,  -10), S!( -23,  -30),
        S!( -35,  -44), S!( -31,  -20), S!( -51,    6), S!( -61,   14), S!( -61,   14), S!( -49,    7), S!( -35,   -7), S!( -39,  -33),
        S!( -31,  -29), S!( -33,   -1), S!( -47,    9), S!( -69,   15), S!( -69,   15), S!( -43,   15), S!( -31,    3), S!( -27,  -21),
        S!( -23,  -17), S!( -33,    3), S!( -39,   11), S!( -67,    5), S!( -65,    5), S!( -35,   29), S!( -33,    3), S!( -23,  -17),
        S!( -27,  -21), S!( -33,   -7), S!( -35,   -3), S!( -49,    5), S!( -51,    1), S!( -39,    3), S!( -39,   -7), S!( -33,  -17),
        S!( -45,  -73), S!( -41,  -39), S!( -41,  -31), S!( -61,  -30), S!( -67,  -28), S!( -49,  -18), S!( -47,  -28), S!( -37,  -38),

    ],
];

#[rustfmt::skip]
pub static mut PIECE_VALUES: [Score; N_PIECES] = [Score::ZERO; N_PIECES];

#[rustfmt::skip]
pub static mut PIECE_TABLES: [[Score; N_SQUARES]; N_PIECES] = [[Score::ZERO; N_SQUARES]; N_PIECES];

#[rustfmt::skip]
pub static mut PAWN_SHIELD_MASKS: [[BitBoard; N_SQUARES]; N_COLORS] = [[BitBoard::ZERO; N_SQUARES]; N_COLORS];

#[inline(always)]
pub fn tempo() -> Score {
    unsafe { TEMPO[0] }
}

#[inline(always)]
pub fn pawn_score(index: usize) -> Score {
    unsafe { PAWN_SCORES[index] }
}

#[inline(always)]
pub fn bishop_score(index: usize) -> Score {
    unsafe { BISHOP_SCORES[index] }
}

#[inline(always)]
pub fn rook_score(index: usize) -> Score {
    unsafe { ROOK_SCORES[index] }
}

#[inline(always)]
pub fn king_score(index: usize) -> Score {
    unsafe { KING_SCORES[index] }
}

#[inline(always)]
pub fn piece_type_value(pt: PieceType) -> Score {
    unsafe { PIECE_TYPE_VALUES[pt.index()] }
}

#[inline(always)]
pub fn piece_score(pc: Piece) -> Score {
    unsafe { PIECE_VALUES[pc.index()] }
}

#[inline(always)]
pub fn piece_sq_value(pc: Piece, sq: SQ) -> Score {
    unsafe { PIECE_TABLES[pc.index()][sq.index()] }
}

#[inline(always)]
pub fn piecetype_sq_value(pt: PieceType, sq: SQ) -> Score {
    unsafe { PIECE_TYPE_TABLES[pt.index()][sq.index()] }
}

#[inline(always)]
pub fn pawns_shield_mask(color: Color, sq: SQ) -> BitBoard {
    unsafe { PAWN_SHIELD_MASKS[color.index()][sq.index()] }
}

fn init_pawn_shields(pawn_shields: &mut [[BitBoard; N_SQUARES]; N_COLORS]) {
    for sq in BitBoard::ALL {
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
    unsafe {
        for pc in Piece::iter(Piece::WhitePawn, Piece::WhiteKing) {
            piece_values[pc.index()] = PIECE_TYPE_VALUES[pc.type_of().index()];
            piece_values[pc.flip().index()] = -PIECE_TYPE_VALUES[pc.type_of().index()];
            for sq in BitBoard::ALL {
                piece_tables[pc.index()][sq.index()] = PIECE_TYPE_TABLES[pc.index()][sq.index()];
                piece_tables[pc.flip().index()][sq.index()] =
                    -PIECE_TYPE_TABLES[pc.index()][sq.square_mirror().index()];
            }
        }
    }
}

pub fn init_eval() {
    unsafe {
        init_piece_values(&mut PIECE_VALUES, &mut PIECE_TABLES);
        init_pawn_shields(&mut PAWN_SHIELD_MASKS);
    }
}
