use crate::types::bitboard::*;
use crate::types::board::*;
use crate::types::color::*;
use crate::types::moov::*;
use crate::types::move_list::*;
use crate::types::piece::*;
use crate::types::square::*;

use super::search::*;
use super::see::*;

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

                let capture_value = see(board, m);

                if capture_value >= 0 {
                    moves.scores[idx] += capture_value + Self::WINNING_CAPTURES_OFFSET;
                } else {
                    moves.scores[idx] += capture_value + Self::LOSING_CAPTURES_OFFSET;
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

    pub fn add_killer(&mut self, board: &Board, m: Move, ply: Ply) {
        let color = board.color_to_play() as usize;
        self.killer_moves[color][ply].rotate_right(1);
        self.killer_moves[color][ply][0] = Some(m);
    }

    pub fn add_history(&mut self, m: Move, depth: Depth) {
        debug_assert!(depth >= 0, "Depth is less than 0 in the history heuristic!");

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
        for killer_move in self.killer_moves[board.color_to_play().index()][ply] {
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
}

impl MoveSorter {
    const N_KILLERS: usize = 3;
    const HASH_MOVE_SCORE: SortValue = 25000;
    const WINNING_CAPTURES_OFFSET: SortValue = 10;
    const QUEEN_PROMOTION_SCORE: SortValue = 8;
    const ROOK_PROMOTION_SCORE: SortValue = 7;
    const BISHOP_PROMOTION_SCORE: SortValue = 6;
    const KNIGHT_PROMOTION_SCORE: SortValue = 5;
    const KILLER_MOVE_SCORE: SortValue = 2;
    const CASTLING_SCORE: SortValue = 1;
    const HISTORY_MOVE_OFFSET: SortValue = -30000;
    const LOSING_CAPTURES_OFFSET: SortValue = -30001;
}
