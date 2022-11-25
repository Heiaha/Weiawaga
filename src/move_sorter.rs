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
    history_scores: [[SortValue; SQ::N_SQUARES]; SQ::N_SQUARES],
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
        let mut m: Move;
        for idx in 0..moves.len() {
            m = moves[idx];

            if Some(m) == hash_move {
                moves.scores[idx] += Self::HASH_MOVE_SCORE;
                continue;
            }

            if m.is_quiet() {
                if self.is_killer(board, m, ply) {
                    moves.scores[idx] += Self::KILLER_MOVE_SCORE;
                    continue;
                }

                if m.is_castling() {
                    moves.scores[idx] += Self::CASTLING_SCORE;
                    continue;
                }

                moves.scores[idx] += Self::HISTORY_MOVE_OFFSET + self.history_score(m);
                continue;
            }

            if m.is_capture() {
                if m.is_ep() {
                    moves.scores[idx] += Self::WINNING_CAPTURES_OFFSET;
                    continue;
                }

                moves.scores[idx] += Self::mvv_lva_score(board, m);

                if Self::see(board, m, -100) {
                    moves.scores[idx] += Self::WINNING_CAPTURES_OFFSET;
                } else {
                    moves.scores[idx] += Self::LOSING_CAPTURES_OFFSET;
                }
            }

            moves.scores[idx] += match m.promotion() {
                PieceType::Knight => Self::KNIGHT_PROMOTION_SCORE,
                PieceType::Bishop => Self::BISHOP_PROMOTION_SCORE,
                PieceType::Rook => Self::ROOK_PROMOTION_SCORE,
                PieceType::Queen => Self::QUEEN_PROMOTION_SCORE,
                PieceType::None => 0,
                _ => unreachable!(),
            };
        }
    }

    #[inline(always)]
    fn mvv_lva_score(board: &Board, m: Move) -> SortValue {
        Self::MVV_LVA_SCORES[board.piece_type_at(m.to_sq()).index() * PieceType::N_PIECE_TYPES
            + board.piece_type_at(m.from_sq()).index()]
    }

    pub fn add_killer(&mut self, board: &Board, m: Move, ply: Ply) {
        let color = board.ctm().index();
        self.killer_moves[color][ply].rotate_right(1);
        self.killer_moves[color][ply][0] = Some(m);
    }

    pub fn add_history(&mut self, m: Move, depth: Depth) {
        let depth = depth as SortValue;
        let from = m.from_sq().index();
        let to = m.to_sq().index();
        self.history_scores[from][to] += depth * depth;

        if self.history_scores[from][to] >= -Self::HISTORY_MOVE_OFFSET {
            for sq1 in Bitboard::ALL {
                for sq2 in Bitboard::ALL {
                    self.history_scores[sq1.index()][sq2.index()] >>= 1; // Divide by two
                }
            }
        }
    }

    fn is_killer(&self, board: &Board, m: Move, ply: usize) -> bool {
        for killer_move in self.killer_moves[board.ctm().index()][ply] {
            if Some(m) == killer_move {
                return true;
            }
        }
        false
    }

    #[inline(always)]
    fn history_score(&self, m: Move) -> SortValue {
        self.history_scores[m.from_sq().index()][m.to_sq().index()]
    }

    pub fn see(board: &Board, m: Move, threshold: SortValue) -> bool {
        if m.promotion() != PieceType::None {
            return true;
        }

        let from_sq = m.from_sq();
        let to_sq = m.to_sq();
        let mut value = Self::SEE_PIECE_TYPE[board.piece_type_at(to_sq).index()] - threshold;

        if value < 0 {
            return false;
        }

        value -= Self::SEE_PIECE_TYPE[board.piece_type_at(from_sq).index()];

        if value >= 0 {
            return true;
        }

        let mut occ = board.all_pieces() ^ from_sq.bb();
        let mut attackers = board.attackers(to_sq, occ);
        let mut stm_attackers;

        let diagonal_sliders = board.diagonal_sliders();
        let orthogonal_sliders = board.orthogonal_sliders();

        let mut ctm = !board.ctm();
        loop {
            attackers &= occ;
            stm_attackers = attackers & board.all_pieces_c(ctm);

            if stm_attackers == Bitboard::ZERO {
                break;
            }

            // We know at this point that there must be a piece, so find the least valuable attacker.
            let attacking_pt = PieceType::iter(PieceType::Pawn, PieceType::King)
                .find(|pt| stm_attackers & board.bitboard_of_pt(*pt) != Bitboard::ZERO)
                .unwrap();

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

            if attacking_pt == PieceType::Pawn
                || attacking_pt == PieceType::Bishop
                || attacking_pt == PieceType::Queen
            {
                attackers |= attacks::bishop_attacks(to_sq, occ) & diagonal_sliders;
            }
            if attacking_pt == PieceType::Rook || attacking_pt == PieceType::Queen {
                attackers |= attacks::rook_attacks(to_sq, occ) & orthogonal_sliders;
            }
        }

        ctm != board.piece_at(from_sq).color_of()
    }
}

impl MoveSorter {
    const N_KILLERS: usize = 3;
    const HASH_MOVE_SCORE: SortValue = 25000;
    const QUEEN_PROMOTION_SCORE: SortValue = 8000;
    const ROOK_PROMOTION_SCORE: SortValue = 7000;
    const BISHOP_PROMOTION_SCORE: SortValue = 6000;
    const KNIGHT_PROMOTION_SCORE: SortValue = 5000;
    const WINNING_CAPTURES_OFFSET: SortValue = 10;
    const KILLER_MOVE_SCORE: SortValue = 2;
    const CASTLING_SCORE: SortValue = 1;
    const HISTORY_MOVE_OFFSET: SortValue = -30000;
    const LOSING_CAPTURES_OFFSET: SortValue = -30001;

    const SEE_PIECE_TYPE: [SortValue; PieceType::N_PIECE_TYPES] = [100, 375, 375, 500, 1025, 10000];

    #[rustfmt::skip]
    const MVV_LVA_SCORES: [SortValue; PieceType::N_PIECE_TYPES * PieceType::N_PIECE_TYPES] = [
        105, 104, 103, 102, 101, 100,
        205, 204, 203, 202, 201, 200,
        305, 304, 303, 302, 301, 300,
        405, 404, 403, 402, 401, 400,
        505, 504, 503, 502, 501, 500,
        605, 604, 603, 602, 601, 600
    ];
}
