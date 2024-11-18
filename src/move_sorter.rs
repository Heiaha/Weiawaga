use super::attacks;
use super::bitboard::*;
use super::board::*;
use super::color::*;
use super::moov::*;
use super::move_list::*;
use super::piece::*;
use super::square::*;
use super::types::*;

pub struct MoveSorter {
    killer_moves: [[[Option<Move>; Self::N_KILLERS]; MAX_MOVES]; Color::N_COLORS],
    history_scores: [[Value; SQ::N_SQUARES]; SQ::N_SQUARES],
}

impl MoveSorter {
    pub fn new() -> Self {
        Self {
            killer_moves: [[[None; Self::N_KILLERS]; MAX_MOVES]; Color::N_COLORS],
            history_scores: [[0; SQ::N_SQUARES]; SQ::N_SQUARES],
        }
    }

    pub fn score_moves(
        &self,
        moves: &mut MoveList,
        board: &Board,
        ply: Ply,
        hash_move: Option<Move>,
    ) {
        for entry in moves.iter_mut() {
            entry.score = self.score_move(entry.m, board, ply, hash_move);
        }
    }

    fn score_move(&self, m: Move, board: &Board, ply: Ply, hash_move: Option<Move>) -> Value {
        if Some(m) == hash_move {
            return Self::HASH_MOVE_SCORE;
        }

        if m.is_quiet() {
            if self.is_killer(board, m, ply) {
                return Self::KILLER_MOVE_SCORE;
            }

            if m.is_castling() {
                return Self::CASTLING_SCORE;
            }

            return Self::HISTORY_MOVE_OFFSET + self.history_score(m);
        }

        let mut score = 0;
        if m.is_capture() {
            if m.is_ep() {
                return Self::WINNING_CAPTURES_OFFSET;
            }

            score += Self::mvv_lva_score(board, m)
                + if Self::see(board, m) {
                    Self::WINNING_CAPTURES_OFFSET
                } else {
                    Self::LOSING_CAPTURES_OFFSET
                };
        }

        score += match m.promotion() {
            Some(PieceType::Knight) => Self::KNIGHT_PROMOTION_SCORE,
            Some(PieceType::Bishop) => Self::BISHOP_PROMOTION_SCORE,
            Some(PieceType::Rook) => Self::ROOK_PROMOTION_SCORE,
            Some(PieceType::Queen) => Self::QUEEN_PROMOTION_SCORE,
            None => 0,
            _ => unreachable!(),
        };
        score
    }

    fn mvv_lva_score(board: &Board, m: Move) -> Value {
        let captured_pt = board
            .piece_type_at(m.to_sq())
            .expect("No captured in MVVLVA.");
        let attacking_pt = board
            .piece_type_at(m.from_sq())
            .expect("No attacker in MVVLVA.");

        Self::MVV_LVA_SCORES[captured_pt.index() * PieceType::N_PIECE_TYPES + attacking_pt.index()]
    }

    pub fn add_killer(&mut self, board: &Board, m: Move, ply: Ply) {
        let color = board.ctm().index();
        let killer_moves = &mut self.killer_moves[color][ply];

        killer_moves.rotate_right(1);
        killer_moves[0] = Some(m);
    }

    pub fn add_history(&mut self, m: Move, depth: Depth) {
        let depth = depth as Value;
        let from = m.from_sq().index();
        let to = m.to_sq().index();
        self.history_scores[from][to] += depth * depth;

        if self.history_scores[from][to] >= -Self::HISTORY_MOVE_OFFSET {
            self.history_scores
                .iter_mut()
                .flatten()
                .for_each(|x| *x >>= 1);
        }
    }

    fn is_killer(&self, board: &Board, m: Move, ply: usize) -> bool {
        self.killer_moves[board.ctm().index()][ply].contains(&Some(m))
    }

    fn history_score(&self, m: Move) -> Value {
        self.history_scores[m.from_sq().index()][m.to_sq().index()]
    }

    pub fn see(board: &Board, m: Move) -> bool {
        if m.promotion().is_some() {
            return true;
        }

        let from_sq = m.from_sq();
        let to_sq = m.to_sq();

        let Some(captured_pt) = board.piece_type_at(to_sq) else {
            return false;
        };

        let mut value = Self::SEE_PIECE_TYPE[captured_pt.index()];

        if value < 0 {
            return false;
        }

        let Some(mut attacking_pt) = board.piece_type_at(from_sq) else {
            return false;
        };

        value -= Self::SEE_PIECE_TYPE[attacking_pt.index()];

        if value >= 0 {
            return true;
        }

        let mut occ = board.all_pieces() ^ from_sq.bb();
        let mut attackers = board.attackers(to_sq, occ);

        let diagonal_sliders = board.diagonal_sliders();
        let orthogonal_sliders = board.orthogonal_sliders();

        let mut ctm = !board.ctm();
        loop {
            attackers &= occ;
            let stm_attackers = attackers & board.all_pieces_c(ctm);

            if stm_attackers == Bitboard::ZERO {
                break;
            }

            // We know at this point that there must be a piece, so find the least valuable attacker.
            attacking_pt = PieceType::iter(PieceType::Pawn, PieceType::King)
                .find(|&pt| stm_attackers & board.bitboard_of_pt(pt) != Bitboard::ZERO)
                .expect("No attacking pt found.");

            ctm = !ctm;

            value = -value - 1 - Self::SEE_PIECE_TYPE[attacking_pt.index()];

            if value >= 0 {
                if attacking_pt == PieceType::King
                    && (attackers & board.all_pieces_c(ctm) != Bitboard::ZERO)
                {
                    ctm = !ctm;
                }
                break;
            }

            occ ^= (stm_attackers & board.bitboard_of_pt(attacking_pt))
                .lsb()
                .bb();

            if matches!(
                attacking_pt,
                PieceType::Pawn | PieceType::Bishop | PieceType::Queen
            ) {
                attackers |= attacks::bishop_attacks(to_sq, occ) & diagonal_sliders;
            }

            if matches!(attacking_pt, PieceType::Rook | PieceType::Queen) {
                attackers |= attacks::rook_attacks(to_sq, occ) & orthogonal_sliders;
            }
        }

        ctm != board
            .piece_at(from_sq)
            .expect("No piece at original attacking square.")
            .color_of()
    }
}

impl MoveSorter {
    const N_KILLERS: usize = 3;
    const HASH_MOVE_SCORE: Value = 25000;
    const QUEEN_PROMOTION_SCORE: Value = 8000;
    const ROOK_PROMOTION_SCORE: Value = 7000;
    const BISHOP_PROMOTION_SCORE: Value = 6000;
    const KNIGHT_PROMOTION_SCORE: Value = 5000;
    const WINNING_CAPTURES_OFFSET: Value = 10;
    const KILLER_MOVE_SCORE: Value = 2;
    const CASTLING_SCORE: Value = 1;
    const HISTORY_MOVE_OFFSET: Value = -30000;
    const LOSING_CAPTURES_OFFSET: Value = -30001;

    const SEE_PIECE_TYPE: [Value; PieceType::N_PIECE_TYPES] = [100, 375, 375, 500, 1025, 10000];

    #[rustfmt::skip]
    const MVV_LVA_SCORES: [Value; PieceType::N_PIECE_TYPES * PieceType::N_PIECE_TYPES] = [
        105, 104, 103, 102, 101, 100,
        205, 204, 203, 202, 201, 200,
        305, 304, 303, 302, 301, 300,
        405, 404, 403, 402, 401, 400,
        505, 504, 503, 502, 501, 500,
        605, 604, 603, 602, 601, 600
    ];
}
