use super::e_constants::*;
use super::score::*;
use crate::types::attacks;
use crate::types::bitboard::*;
use crate::types::board::*;
use crate::types::color::*;
use crate::types::file::*;
use crate::types::piece::*;
use crate::types::square::*;

pub struct Evaluator {
    color: Color,
    our_king: SQ,
    their_king: SQ,
    our_pawns: BitBoard,
    their_pawns: BitBoard,
    all_pieces: BitBoard,
}

impl Evaluator {

    pub fn new(board: &Board, color: Color) -> Evaluator {
        Evaluator {
            color,
            our_king: board.bitboard_of(color, PieceType::King).lsb(),
            their_king: board.bitboard_of(!color, PieceType::King).lsb(),
            our_pawns: board.bitboard_of(color, PieceType::Pawn),
            their_pawns: board.bitboard_of(!color, PieceType::Pawn),
            all_pieces: board.all_pieces(Color::White) | board.all_pieces(Color::Black),
        }
    }

    ////////////////////////////////////////////////////////////////
    // PAWN
    ////////////////////////////////////////////////////////////////

    fn n_passed_pawns(&self) -> Value {
        let mut fill = BitBoard::ZERO;
        fill |= self.their_pawns.shift(Direction::SouthWest.relative(self.color), 1);
        fill |= self.their_pawns.shift(Direction::SouthEast.relative(self.color), 1);
        fill = fill.fill(Direction::South.relative(self.color));
        (!fill & self.our_pawns).pop_count()
    }

    fn n_doubled_pawns(&self) -> Value {
        let mut fill = self.our_pawns.shift(Direction::North, 1);
        fill = fill.fill(Direction::North);
        (fill & self.our_pawns).pop_count()
    }

    fn n_isolated_pawns(&self) -> Value {
        ((self.our_pawns & !self.our_pawns.shift(Direction::East, 1).file_fill()) &
            (self.our_pawns & !self.our_pawns.shift(Direction::West, 1).file_fill())).pop_count()
    }

    pub fn pawn_score(&self, board: &Board) -> Score {
        let mut score = Score::ZERO;
        score += PAWN_SCORES[IX_PASSED_PAWN_VALUE] * self.n_passed_pawns();
        score += PAWN_SCORES[IX_DOUBLED_PAWN_PENALTY] * self.n_doubled_pawns();
        score += PAWN_SCORES[IX_ISOLATED_PAWN_PENALTY] * self.n_isolated_pawns();
        score
    }

////////////////////////////////////////////////////////////////
// BISHOP
////////////////////////////////////////////////////////////////

    fn has_bishop_pair(&self, bishop_bb: BitBoard) -> bool {
        (bishop_bb & BitBoard::LIGHT_SQUARES) != BitBoard::ZERO && (bishop_bb & BitBoard::DARK_SQUARES) != BitBoard::ZERO
    }

    fn pawns_on_same_color_square(&self, board: &Board, sq: SQ) -> Value {
        (board.bitboard_of(self.color, PieceType::Pawn) & if sq.bb() & BitBoard::DARK_SQUARES != BitBoard::ZERO { BitBoard::DARK_SQUARES } else { BitBoard::LIGHT_SQUARES }).pop_count()
    }

    fn bishop_score(&self, board: &Board) -> Score {
        let mut score = Score::ZERO;
        let bishops_bb = board.bitboard_of(self.color, PieceType::Bishop);

        if self.has_bishop_pair(bishops_bb) {
            score += BISHOP_SCORES[IX_BISHOP_PAIR_VALUE];
        }

        let mut attacks: BitBoard;
        for sq in bishops_bb {
            attacks = attacks::bishop_attacks(sq, self.all_pieces) & !self.all_pieces;
            if (attacks & BitBoard::CENTER).pop_count() == 2 {
                score += BISHOP_SCORES[IX_BISHOP_ATTACKS_CENTER];
            }
            score += BISHOP_SCORES[IX_BISHOP_SAME_COLOR_PAWN_PENALTY] * self.pawns_on_same_color_square(board, sq);
        }
        score
    }

////////////////////////////////////////////////////////////////
// ROOK
////////////////////////////////////////////////////////////////

    fn rook_score(&self, board: &Board) -> Score {
        let mut score = Score::ZERO;
        let rooks_bb = board.bitboard_of(self.color, PieceType::Rook);

        let mut rook_file_bb: BitBoard;
        let mut piece_mobility: Value;
        for sq in rooks_bb {
            rook_file_bb = sq.file().bb();

            if self.our_pawns & rook_file_bb == BitBoard::ZERO {
                if self.their_pawns & rook_file_bb == BitBoard::ZERO {
                    score += ROOK_SCORES[IX_ROOK_ON_OPEN_FILE];
                } else {
                    score += ROOK_SCORES[IX_ROOK_ON_SEMIOPEN_FILE];
                }
            }

            piece_mobility = (attacks::rook_attacks(sq, self.all_pieces) & !self.all_pieces).pop_count();
            if piece_mobility <= 3 {
                let kf = self.our_king.file();
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

    fn pawns_shielding_king(&self) -> Value {
        unsafe { (PAWN_SHIELD_MASKS[self.color.index()][self.our_king.index()] & self.our_pawns).pop_count() }
    }

    fn king_score(&self) -> Score {
        KING_SCORES[IX_KING_PAWN_SHIELD_BONUS] * self.pawns_shielding_king()
    }
}



pub fn eval(board: &Board) -> Value {

    let mut score = Score::ZERO;
    let white_evaluator = Evaluator::new(board, Color::White);
    let black_evaluator = Evaluator::new(board, Color::Black);
    score += board.p_sq_score() + board.material_score();
    score += if board.color_to_play() == Color::White { TEMPO[0] } else { -TEMPO[0] };

    score += white_evaluator.pawn_score(board);
    score -= black_evaluator.pawn_score(board);

    score += white_evaluator.bishop_score(board);
    score -= black_evaluator.bishop_score(board);

    score += white_evaluator.rook_score(board);
    score -= black_evaluator.rook_score(board);

    score += white_evaluator.king_score();
    score -= black_evaluator.king_score();

    if board.color_to_play() == Color::White {
        score.eval(board.phase())
    } else {
        -score.eval(board.phase())
    }
}
