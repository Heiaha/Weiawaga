use super::score::Score;
use crate::types::bitboard::*;
use crate::types::color::*;
use crate::types::piece::*;
use crate::types::square::*;

#[rustfmt::skip]
pub static mut TEMPO: [Score; 1] = [S!(  19,   18), ];

#[rustfmt::skip]
pub static mut PIECE_TYPE_VALUES: [Score; PieceType::N_PIECE_TYPES] = [
    S!( 113,  139), S!( 358,  332), S!( 362,  332), S!( 475,  561), S!(1010, 1020), S!(   0,    0),
];

//pawn scoring
#[rustfmt::skip]
pub static mut PAWN_SCORES: [Score; 3] = [S!( -11,   17), S!( -12,  -20), S!( -19,    0), ];
pub const IX_PASSED_PAWN_VALUE: usize = 0;
pub const IX_DOUBLED_PAWN_PENALTY: usize = 1;
pub const IX_ISOLATED_PAWN_PENALTY: usize = 2;

//bishop scoring
#[rustfmt::skip]
pub static mut BISHOP_SCORES: [Score; 3] = [S!(  -4,   -7), S!(  34,    9), S!(  36,   49), ];
pub const IX_BISHOP_SAME_COLOR_PAWN_PENALTY: usize = 0; // per pawn
pub const IX_BISHOP_ATTACKS_CENTER: usize = 1;
pub const IX_BISHOP_PAIR_VALUE: usize = 2;

//rook scoring
#[rustfmt::skip]
pub static mut ROOK_SCORES: [Score; 3] = [S!( -20,  -31), S!(  45,   14), S!(  23,   11), ];
pub const IX_KING_TRAPPING_ROOK_PENALTY: usize = 0;
pub const IX_ROOK_ON_OPEN_FILE: usize = 1;
pub const IX_ROOK_ON_SEMIOPEN_FILE: usize = 2;

//king scoring
#[rustfmt::skip]
pub static mut KING_SCORES: [Score; 1] = [S!(  22,   -9), ];
pub const IX_KING_PAWN_SHIELD_BONUS: usize = 0;

#[rustfmt::skip]
pub static mut PIECE_TYPE_TABLES: [[Score; SQ::N_SQUARES]; PieceType::N_PIECE_TYPES] = [
    [
        S!(   0,    0), S!(   0,    0), S!(   0,    0), S!(   0,    0), S!(   0,    0), S!(   0,    0), S!(   0,    0), S!(   0,    0),
        S!( -44,  -33), S!( -35,  -37), S!( -37,  -34), S!( -54,  -31), S!( -44,  -17), S!( -12,  -37), S!(  -7,  -47), S!( -39,  -49),
        S!( -39,  -41), S!( -40,  -41), S!( -29,  -47), S!( -33,  -33), S!( -19,  -39), S!( -14,  -41), S!(  -3,  -54), S!( -16,  -55),
        S!( -40,  -32), S!( -34,  -36), S!( -31,  -42), S!( -11,  -50), S!( -10,  -52), S!(  -5,  -50), S!( -22,  -42), S!( -27,  -48),
        S!( -23,  -14), S!( -17,  -26), S!(  -9,  -33), S!(  -2,  -49), S!(  12,  -46), S!(  10,  -42), S!(  -4,  -30), S!( -24,  -28),
        S!(   7,   45), S!(   2,   45), S!(  22,   37), S!(  22,   23), S!(  32,   13), S!(  30,    7), S!(  10,   33), S!(  -4,   33),
        S!(  64,  107), S!(  64,  107), S!(  52,   87), S!(  52,   83), S!(  44,   83), S!(  40,   81), S!(  32,   89), S!(  28,  107),
        S!(   0,    0), S!(   0,    0), S!(   0,    0), S!(   0,    0), S!(   0,    0), S!(   0,    0), S!(   0,    0), S!(   0,    0),
    ],
    [
        S!(-105,  -70), S!( -55,  -91), S!( -70,  -55), S!( -60,  -53), S!( -46,  -59), S!( -52,  -59), S!( -48,  -83), S!( -68,  -93),
        S!( -65,  -75), S!( -63,  -57), S!( -38,  -45), S!( -32,  -40), S!( -27,  -42), S!( -14,  -51), S!( -40,  -61), S!( -36,  -71),
        S!( -56,  -55), S!( -33,  -40), S!( -16,  -41), S!( -16,  -24), S!(  -4,  -20), S!( -10,  -44), S!(  -5,  -52), S!( -34,  -61),
        S!( -36,  -47), S!( -20,  -38), S!(  -3,  -17), S!(  -4,  -16), S!(   8,  -17), S!(   1,  -24), S!(   5,  -33), S!( -22,  -58),
        S!( -24,  -50), S!( -11,  -28), S!(  13,  -18), S!(  38,  -17), S!(  18,  -17), S!(  36,  -16), S!(   6,  -32), S!( -12,  -49),
        S!( -47,  -69), S!(   9,  -43), S!(  23,  -23), S!(  32,  -24), S!(  32,  -28), S!(  27,  -29), S!(  17,  -47), S!( -15,  -73),
        S!( -67,  -75), S!( -37,  -55), S!(  13,  -49), S!(   9,  -41), S!(  -9,  -47), S!(  17,  -47), S!( -18,  -63), S!( -38,  -87),
        S!( -88, -101), S!( -76,  -81), S!( -64,  -53), S!( -40,  -71), S!( -30,  -57), S!( -66,  -79), S!( -64,  -87), S!( -88, -101),
    ],
    [
        S!( -47,  -38), S!( -17,  -15), S!( -32,  -23), S!( -32,  -13), S!( -28,  -17), S!( -27,  -20), S!( -35,  -26), S!( -43,  -44),
        S!(  -7,  -30), S!( -10,  -27), S!(   5,  -18), S!( -16,   -4), S!( -12,   -6), S!(   2,  -14), S!(  12,  -27), S!( -12,  -34),
        S!( -13,  -16), S!(  -3,   -6), S!(  -5,  -11), S!(  -1,    3), S!(  -1,    9), S!(  -4,  -17), S!(  -2,  -10), S!(  -4,  -20),
        S!( -26,  -15), S!(  -6,    1), S!(  -2,   10), S!(  18,   10), S!(  18,    2), S!(   0,    6), S!(  -4,   -8), S!( -16,  -20),
        S!( -16,  -11), S!(  -9,    6), S!(  12,    4), S!(  23,   10), S!(  23,    8), S!(  18,    5), S!(  -2,   -1), S!( -12,  -10),
        S!( -32,  -10), S!(  10,   -8), S!(  -1,  -17), S!(  10,    0), S!(  10,   -6), S!(   3,  -15), S!(   2,   -2), S!( -14,   -8),
        S!( -30,  -20), S!( -20,  -24), S!( -18,   -4), S!( -18,  -20), S!(   2,  -12), S!(   0,  -12), S!( -14,  -26), S!( -28,  -32),
        S!( -42,  -44), S!( -26,  -22), S!( -30,  -30), S!( -32,  -22), S!( -30,  -20), S!( -34,  -24), S!( -22,  -24), S!( -46,  -44),
    ],
    [
        S!( -38,  -27), S!( -43,  -21), S!( -39,  -23), S!( -27,  -32), S!( -27,  -34), S!( -25,  -27), S!( -35,  -18), S!( -25,  -40),
        S!( -60,  -28), S!( -49,  -33), S!( -47,  -27), S!( -45,  -29), S!( -41,  -37), S!( -25,  -43), S!( -29,  -33), S!( -48,  -36),
        S!( -50,  -28), S!( -45,  -26), S!( -49,  -30), S!( -49,  -28), S!( -44,  -30), S!( -28,  -33), S!( -20,  -32), S!( -37,  -33),
        S!( -53,  -20), S!( -50,  -23), S!( -42,  -19), S!( -31,  -25), S!( -28,  -31), S!( -34,  -27), S!( -18,  -31), S!( -37,  -27),
        S!( -37,  -15), S!( -30,  -16), S!( -22,  -19), S!( -16,  -24), S!( -21,  -28), S!( -13,  -21), S!( -23,  -21), S!( -26,  -21),
        S!( -33,  -12), S!( -12,  -14), S!( -15,  -18), S!( -14,  -20), S!( -16,  -27), S!(  -5,  -19), S!(  -3,  -13), S!( -18,  -21),
        S!( -13,  -13), S!( -12,   -8), S!(   0,  -12), S!(   4,  -15), S!(   4,  -22), S!(  10,  -13), S!( -10,   -9), S!(  -1,  -14),
        S!(  -8,  -13), S!(  -6,  -16), S!( -11,  -16), S!(  -3,  -21), S!(  -3,  -18), S!(  -3,  -16), S!( -11,  -17), S!(  -9,  -18),
    ],
    [
        S!( -25,  -32), S!( -29,  -33), S!( -21,  -32), S!(  -2,  -30), S!( -17,  -25), S!( -26,  -38), S!( -28,  -38), S!( -34,  -44),
        S!( -24,  -38), S!( -12,  -26), S!(   0,  -28), S!(  -2,  -24), S!(  -2,  -22), S!(   0,  -28), S!( -12,  -28), S!( -10,  -30),
        S!( -20,  -24), S!(  -9,  -21), S!(  -7,   -6), S!(  -7,  -11), S!(  -5,  -10), S!(  -1,   -3), S!(   8,  -10), S!(  -4,  -20),
        S!( -14,  -24), S!( -16,   -6), S!( -11,   -5), S!(  -5,   11), S!(   3,    8), S!(  -1,    6), S!(   6,   -1), S!(  -5,   -5),
        S!( -21,  -21), S!( -10,  -14), S!(  -7,   -5), S!(  -1,    5), S!(  15,    7), S!(  19,    5), S!(  12,    0), S!(   5,   -5),
        S!( -20,  -26), S!( -10,  -18), S!(   5,   -9), S!(  13,    5), S!(  21,    5), S!(  21,    5), S!(  16,    0), S!(   6,  -10),
        S!( -22,  -28), S!( -18,  -18), S!(  -4,   -2), S!(   4,    0), S!(   2,    0), S!(  12,    0), S!(  12,   -2), S!(   2,  -10),
        S!( -36,  -34), S!( -12,  -16), S!(  -4,  -10), S!(  -1,   -5), S!(   3,   -5), S!(  -2,  -10), S!(  -2,  -10), S!( -12,  -20),
    ],
    [
        S!(   7,  -76), S!(  21,  -54), S!(  -1,  -38), S!( -20,  -47), S!(  10,  -48), S!( -13,  -44), S!(  21,  -56), S!(  19,  -76),
        S!(   5,  -50), S!(  10,  -35), S!( -20,  -17), S!( -26,  -19), S!( -26,  -15), S!( -26,  -15), S!(  16,  -33), S!(  12,  -45),
        S!( -22,  -43), S!( -30,  -19), S!( -44,   -7), S!( -46,    1), S!( -46,    3), S!( -46,   -3), S!( -18,  -15), S!( -28,  -32),
        S!( -38,  -40), S!( -32,  -18), S!( -54,    6), S!( -66,   18), S!( -66,   17), S!( -54,    8), S!( -40,   -7), S!( -44,  -31),
        S!( -36,  -29), S!( -38,   -1), S!( -50,   11), S!( -72,   17), S!( -70,   17), S!( -46,   15), S!( -34,    5), S!( -32,  -21),
        S!( -28,  -19), S!( -34,    3), S!( -40,   11), S!( -68,    7), S!( -66,    7), S!( -36,   29), S!( -34,    5), S!( -24,  -17),
        S!( -32,  -23), S!( -34,   -7), S!( -38,   -3), S!( -50,    3), S!( -52,    3), S!( -40,    5), S!( -40,   -5), S!( -34,  -17),
        S!( -48,  -75), S!( -44,  -41), S!( -46,  -33), S!( -64,  -28), S!( -68,  -28), S!( -50,  -18), S!( -48,  -26), S!( -42,  -40),
    ],
];

#[rustfmt::skip]
pub static mut PIECE_VALUES: [Score; Piece::N_PIECES] = [Score::ZERO; Piece::N_PIECES];

#[rustfmt::skip]
pub static mut PIECE_TABLES: [[Score; SQ::N_SQUARES]; Piece::N_PIECES] = [[Score::ZERO; SQ::N_SQUARES]; Piece::N_PIECES];

#[rustfmt::skip]
pub static mut PAWN_SHIELD_MASKS: [[BitBoard; SQ::N_SQUARES]; Color::N_COLORS] = [[BitBoard::ZERO; SQ::N_SQUARES]; Color::N_COLORS];

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

fn init_pawn_shields(pawn_shields: &mut [[BitBoard; SQ::N_SQUARES]; Color::N_COLORS]) {
    for sq in BitBoard::ALL {
        let sq_bb = sq.bb();
        pawn_shields[Color::White.index()][sq.index()] = sq_bb.shift(Direction::North)
            | sq_bb.shift(Direction::NorthEast)
            | sq_bb.shift(Direction::NorthWest);
        pawn_shields[Color::Black.index()][sq.index()] = sq_bb.shift(Direction::South)
            | sq_bb.shift(Direction::SouthEast)
            | sq_bb.shift(Direction::SouthWest);
    }
}

fn init_piece_values(
    piece_values: &mut [Score; Piece::N_PIECES],
    piece_tables: &mut [[Score; SQ::N_SQUARES]; Piece::N_PIECES],
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
