use super::e_constants::*;
use super::score::*;
use crate::types::attacks;
use crate::types::bitboard::*;
use crate::types::board::*;
use crate::types::color::*;
use crate::types::file::*;
use crate::types::piece::*;
use crate::types::square::*;

////////////////////////////////////////////////////////////////
// PAWN
////////////////////////////////////////////////////////////////

fn n_passed_pawns(board: &Board, color: Color) -> Value {
    let our_pawns = board.bitboard_of(color, PieceType::Pawn);
    let their_pawns = board.bitboard_of(!color, PieceType::Pawn);
    let mut fill = BitBoard::ZERO;
    fill |= their_pawns.shift(Direction::SouthWest.relative(color), 1);
    fill |= their_pawns.shift(Direction::SouthEast.relative(color), 1);
    fill = fill.fill(Direction::South.relative(color));
    (!fill & our_pawns).pop_count()
}

fn n_doubled_pawns(board: &Board, color: Color) -> Value {
    let pawns_bb = board.bitboard_of(color, PieceType::Pawn);
    let mut fill = pawns_bb.shift(Direction::North, 1);
    fill = fill.fill(Direction::North);
    (fill & pawns_bb).pop_count()
}

fn n_isolated_pawns(board: &Board, color: Color) -> Value {
    let pawns_bb = board.bitboard_of(color, PieceType::Pawn);
    ((pawns_bb & !pawns_bb.shift(Direction::East, 1).file_fill()) & (pawns_bb & !pawns_bb.shift(Direction::West, 1).file_fill())).pop_count()
}

pub fn pawn_score(board: &Board, color: Color) -> Score {
    let mut score = Score::ZERO;
    score += PAWN_SCORES[IX_PASSED_PAWN_VALUE] * n_passed_pawns(board, color);
    score += PAWN_SCORES[IX_DOUBLED_PAWN_PENALTY] * n_doubled_pawns(board, color);
    score += PAWN_SCORES[IX_ISOLATED_PAWN_PENALTY] * n_isolated_pawns(board, color);
    score
}

////////////////////////////////////////////////////////////////
// BISHOP
////////////////////////////////////////////////////////////////

fn has_bishop_pair(bishop_bb: BitBoard) -> bool {
    (bishop_bb & BitBoard::LIGHT_SQUARES) != BitBoard::ZERO && (bishop_bb & BitBoard::DARK_SQUARES) != BitBoard::ZERO
}

fn pawns_on_same_color_square(board: &Board, sq: SQ, color: Color) -> Value {
    (board.bitboard_of(color, PieceType::Pawn) & if sq.bb() & BitBoard::DARK_SQUARES != BitBoard::ZERO { BitBoard::DARK_SQUARES } else { BitBoard::LIGHT_SQUARES }).pop_count()
}

fn bishop_score(board: &Board, color: Color, all_pieces: BitBoard) -> Score {
    let mut score = Score::ZERO;
    let bishops_bb = board.bitboard_of(color, PieceType::Bishop);

    if has_bishop_pair(bishops_bb) {
        score += BISHOP_SCORES[IX_BISHOP_PAIR_VALUE];
    }

    let mut attacks: BitBoard;
    for sq in bishops_bb {
        attacks = attacks::bishop_attacks(sq, all_pieces) & !all_pieces;
        if (attacks & BitBoard::CENTER).pop_count() == 2 {
            score += BISHOP_SCORES[IX_BISHOP_ATTACKS_CENTER];
        }
        score += BISHOP_SCORES[IX_BISHOP_SAME_COLOR_PAWN_PENALTY] * pawns_on_same_color_square(board, sq, color);
    }
    score
}

////////////////////////////////////////////////////////////////
// ROOK
////////////////////////////////////////////////////////////////

fn rook_score(board: &Board, color: Color, all_pieces: BitBoard) -> Score {
    let mut score = Score::ZERO;
    let rooks_bb = board.bitboard_of(color, PieceType::Rook);
    let our_king_sq = board.bitboard_of(color, PieceType::King).lsb();

    let our_pawns = board.bitboard_of(color, PieceType::Pawn);
    let their_pawns = board.bitboard_of(!color, PieceType::Pawn);

    let mut rook_file_bb: BitBoard;
    let mut piece_mobility: Value;
    for sq in rooks_bb {
        rook_file_bb = sq.file().bb();

        if our_pawns & rook_file_bb == BitBoard::ZERO {
            if their_pawns & rook_file_bb == BitBoard::ZERO {
                score += ROOK_SCORES[IX_ROOK_ON_OPEN_FILE];
            } else {
                score += ROOK_SCORES[IX_ROOK_ON_SEMIOPEN_FILE];
            }
        }

        piece_mobility = (attacks::rook_attacks(sq, all_pieces) & !all_pieces).pop_count();
        if piece_mobility <= 3 {
            let kf = our_king_sq.file();
            if (kf < File::E) == (sq.file() < kf) {
                score += ROOK_SCORES[IX_KING_TRAPPING_ROOK_PENALTY];
            }
        }
    }
    score
}

////////////////////////////////////////////////////////////////
// KING
////////////////////////////////////////////////////////////////

fn pawns_shielding_king(board: &Board, color: Color) -> Value {
    let king_sq = board.bitboard_of(color, PieceType::King).lsb();
    let pawns = board.bitboard_of(color, PieceType::Pawn);
    unsafe { (PAWN_SHIELD_MASKS[color.index()][king_sq.index()] & pawns).pop_count() }
}

fn king_score(board: &Board, color: Color) -> Score {
    KING_SCORES[IX_KING_PAWN_SHIELD_BONUS] * pawns_shielding_king(board, color)
}

pub fn eval(board: &Board) -> Value {
    let white_pieces = board.all_pieces(Color::White);
    let black_pieces = board.all_pieces(Color::Black);
    let all_pieces = white_pieces | black_pieces;

    let mut score = Score::ZERO;
    score += board.p_sq_score() + board.material_score();
    score += if board.color_to_play() == Color::White { TEMPO[0] } else { -TEMPO[0] };

    score += pawn_score(board, Color::White);
    score -= pawn_score(board, Color::Black);

    score += bishop_score(board, Color::White, all_pieces);
    score -= bishop_score(board, Color::Black, all_pieces);

    score += rook_score(board, Color::White, all_pieces);
    score -= rook_score(board, Color::Black, all_pieces);

    score += king_score(board, Color::White);
    score -= king_score(board, Color::Black);

    if board.color_to_play() == Color::White {
        score.eval(board.phase())
    } else {
        -score.eval(board.phase())
    }
}
