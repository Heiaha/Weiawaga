use super::attacks;
use super::bitboard::*;
use super::board::*;
use super::moov::*;
use super::move_list::*;
use super::piece::*;
use super::square::*;
use super::types::*;

pub struct MoveSorter {
    killer_moves: [Option<Move>; MAX_MOVES],
    history_scores: SQMap<SQMap<Value>>,
}

impl MoveSorter {
    pub fn new() -> Self {
        Self {
            killer_moves: [None; MAX_MOVES],
            history_scores: SQMap::new([SQMap::new([0; SQ::N_SQUARES]); SQ::N_SQUARES]),
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
            if self.is_killer(m, ply) {
                return Self::KILLER_MOVE_SCORE;
            }

            if m.is_castling() {
                return Self::CASTLING_SCORE;
            }

            return self.history_score(m);
        }

        let mut score = 0;
        if m.is_capture() {
            if m.is_ep() {
                return Self::CAPTURE_SCORE;
            }

            score += Self::mvv_lva_score(board, m)
                + if Self::see(board, m) {
                    Self::CAPTURE_SCORE
                } else {
                    -Self::CAPTURE_SCORE
                };
        }

        score += match m.promotion() {
            Some(pt) => Self::PROMOTION_SCORE + Self::SEE_PIECE_TYPE[pt],
            None => 0,
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

    pub fn add_killer(&mut self, m: Move, ply: Ply) {
        self.killer_moves[ply] = Some(m);
    }

    pub fn add_history(&mut self, m: Move, depth: Depth) {
        let depth = depth as Value;
        let from = m.from_sq();
        let to = m.to_sq();
        self.history_scores[from][to] += depth * depth;

        if self.history_scores[from][to] >= Self::HISTORY_MAX {
            self.history_scores
                .iter_mut()
                .flatten()
                .for_each(|x| *x >>= 1);
        }
    }

    fn is_killer(&self, m: Move, ply: Ply) -> bool {
        self.killer_moves[ply] == Some(m)
    }

    fn history_score(&self, m: Move) -> Value {
        self.history_scores[m.from_sq()][m.to_sq()]
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

        let mut value = Self::SEE_PIECE_TYPE[captured_pt];

        if value < 0 {
            return false;
        }

        let Some(mut attacking_pt) = board.piece_type_at(from_sq) else {
            return false;
        };

        value -= Self::SEE_PIECE_TYPE[attacking_pt];

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

            value = -value - 1 - Self::SEE_PIECE_TYPE[attacking_pt];

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
    const HISTORY_MAX: Value = i16::MAX as Value / 2;
    const HASH_MOVE_SCORE: Value = 100 * Self::HISTORY_MAX;
    const PROMOTION_SCORE: Value = 50 * Self::HISTORY_MAX;
    const CAPTURE_SCORE: Value = 10 * Self::HISTORY_MAX;
    const KILLER_MOVE_SCORE: Value = 5 * Self::HISTORY_MAX;
    const CASTLING_SCORE: Value = 2 * Self::HISTORY_MAX;

    const SEE_PIECE_TYPE: PieceTypeMap<Value> =
        PieceTypeMap::new([100, 375, 375, 500, 1025, 10000]);

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
