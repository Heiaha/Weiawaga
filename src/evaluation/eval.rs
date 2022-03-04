use super::e_constants::*;
use super::score::*;
use crate::types::attacks;
use crate::types::bitboard::*;
use crate::types::board::*;
use crate::types::color::*;
use crate::types::file::*;
use crate::types::piece::*;
use crate::types::square::*;

pub struct Evaluator<'a> {
    board: &'a Board,
    color: Color,
    our_king: SQ,
    our_pawns: Bitboard,
    their_pawns: Bitboard,
    all_pieces: Bitboard,
}

impl<'a> Evaluator<'a> {
    pub fn new(board: &'a Board, color: Color) -> Self {
        Self {
            board,
            color,
            our_king: board.bitboard_of(color, PieceType::King).lsb(),
            our_pawns: board.bitboard_of(color, PieceType::Pawn),
            their_pawns: board.bitboard_of(!color, PieceType::Pawn),
            all_pieces: board.all_pieces(),
        }
    }

    ////////////////////////////////////////////////////////////////
    // PAWN
    ////////////////////////////////////////////////////////////////

    fn n_passed_pawns(&self) -> Value {
        let mut fill = Bitboard::ZERO;
        fill |= self
            .their_pawns
            .shift(Direction::SouthWest.relative(self.color));
        fill |= self
            .their_pawns
            .shift(Direction::SouthEast.relative(self.color));
        fill = fill.fill(Direction::South.relative(self.color));
        (!fill & self.our_pawns).pop_count()
    }

    fn n_doubled_pawns(&self) -> Value {
        let mut fill = self.our_pawns.shift(Direction::North);
        fill = fill.fill(Direction::North);
        (fill & self.our_pawns).pop_count()
    }

    fn n_isolated_pawns(&self) -> Value {
        ((self.our_pawns & !self.our_pawns.shift(Direction::East).file_fill())
            & (self.our_pawns & !self.our_pawns.shift(Direction::West).file_fill()))
        .pop_count()
    }

    pub fn pawn_eval(&self) -> Score {
        let mut score = Score::ZERO;
        score += pawn_score(IX_PASSED_PAWN_VALUE) * self.n_passed_pawns();
        score += pawn_score(IX_DOUBLED_PAWN_PENALTY) * self.n_doubled_pawns();
        score += pawn_score(IX_ISOLATED_PAWN_PENALTY) * self.n_isolated_pawns();

        score
    }

    ////////////////////////////////////////////////////////////////
    // BISHOP
    ////////////////////////////////////////////////////////////////

    fn has_bishop_pair(&self, bishop_bb: Bitboard) -> bool {
        (bishop_bb & Bitboard::LIGHT_SQUARES) != Bitboard::ZERO
            && (bishop_bb & Bitboard::DARK_SQUARES) != Bitboard::ZERO
    }

    fn pawns_on_same_color_square(&self, sq: SQ) -> Value {
        (self.board.bitboard_of(self.color, PieceType::Pawn)
            & if sq.bb() & Bitboard::DARK_SQUARES != Bitboard::ZERO {
                Bitboard::DARK_SQUARES
            } else {
                Bitboard::LIGHT_SQUARES
            })
        .pop_count()
    }

    fn bishop_eval(&self) -> Score {
        let mut score = Score::ZERO;
        let bishops_bb = self.board.bitboard_of(self.color, PieceType::Bishop);

        if self.has_bishop_pair(bishops_bb) {
            score += bishop_score(IX_BISHOP_PAIR_VALUE);
        }

        let mut attacks: Bitboard;
        for sq in bishops_bb {
            attacks = attacks::bishop_attacks(sq, self.all_pieces) & !self.all_pieces;
            if (attacks & Bitboard::CENTER).pop_count() == 2 {
                score += bishop_score(IX_BISHOP_ATTACKS_CENTER);
            }
            score += bishop_score(IX_BISHOP_SAME_COLOR_PAWN_PENALTY)
                * self.pawns_on_same_color_square(sq);
        }
        score
    }

    ////////////////////////////////////////////////////////////////
    // ROOK
    ////////////////////////////////////////////////////////////////

    fn rook_eval(&self) -> Score {
        let mut score = Score::ZERO;
        let rooks_bb = self.board.bitboard_of(self.color, PieceType::Rook);

        let mut rook_file_bb: Bitboard;
        let mut piece_mobility: Value;
        for sq in rooks_bb {
            rook_file_bb = sq.file().bb();

            if self.our_pawns & rook_file_bb == Bitboard::ZERO {
                if self.their_pawns & rook_file_bb == Bitboard::ZERO {
                    score += rook_score(IX_ROOK_ON_OPEN_FILE);
                } else {
                    score += rook_score(IX_ROOK_ON_SEMIOPEN_FILE);
                }
            }

            piece_mobility =
                (attacks::rook_attacks(sq, self.all_pieces) & !self.all_pieces).pop_count();
            if piece_mobility <= 3 {
                let kf = self.our_king.file();
                if (kf < File::E) == (sq.file() < kf) {
                    score += rook_score(IX_KING_TRAPPING_ROOK_PENALTY);
                }
            }
        }

        score
    }

    ////////////////////////////////////////////////////////////////
    // KING
    ////////////////////////////////////////////////////////////////

    fn pawns_shielding_king(&self) -> Value {
        (pawns_shield_mask(self.color, self.our_king) & self.our_pawns).pop_count()
    }

    fn king_eval(&self) -> Score {
        king_score(IX_KING_PAWN_SHIELD_BONUS) * self.pawns_shielding_king()
    }

    fn p_sq_eval(&self) -> Score {
        let mut score = Score::ZERO;

        for pt in PieceType::iter(PieceType::Pawn, PieceType::King) {
            let bb = self.board.bitboard_of(self.color, pt);
            for sq in bb {
                score += piecetype_sq_value(pt, sq.relative(self.color));
            }
        }
        score
    }

    fn material_eval(&self) -> Score {
        let mut score = Score::ZERO;
        for pt in PieceType::iter(PieceType::Pawn, PieceType::Queen) {
            let bb = self.board.bitboard_of(self.color, pt);
            score += piece_type_value(pt) * bb.pop_count();
        }
        score
    }
}

pub fn eval(board: &Board) -> Value {
    let mut score = Score::ZERO;
    let white_evaluator = Evaluator::new(board, Color::White);
    let black_evaluator = Evaluator::new(board, Color::Black);
    score += board.p_sq_score() + board.material_score();

    score += if board.color_to_play() == Color::White {
        tempo()
    } else {
        -tempo()
    };

    score += white_evaluator.pawn_eval();
    score -= black_evaluator.pawn_eval();

    score += white_evaluator.bishop_eval();
    score -= black_evaluator.bishop_eval();

    score += white_evaluator.rook_eval();
    score -= black_evaluator.rook_eval();

    score += white_evaluator.king_eval();
    score -= black_evaluator.king_eval();

    if board.color_to_play() == Color::White {
        score.eval(board.phase())
    } else {
        -score.eval(board.phase())
    }
}

pub fn tune_eval(board: &Board) -> Value {
    let mut score = Score::ZERO;
    let white_evaluator = Evaluator::new(board, Color::White);
    let black_evaluator = Evaluator::new(board, Color::Black);

    score += white_evaluator.material_eval();
    score -= black_evaluator.material_eval();

    score += white_evaluator.p_sq_eval();
    score -= black_evaluator.p_sq_eval();

    score += if board.color_to_play() == Color::White {
        tempo()
    } else {
        -tempo()
    };

    score += white_evaluator.pawn_eval();
    score -= black_evaluator.pawn_eval();

    score += white_evaluator.bishop_eval();
    score -= black_evaluator.bishop_eval();

    score += white_evaluator.rook_eval();
    score -= black_evaluator.rook_eval();

    score += white_evaluator.king_eval();
    score -= black_evaluator.king_eval();

    score.eval(board.phase())
}
