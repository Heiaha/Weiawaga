use super::search::*;
use super::see::*;
use crate::types::bitboard::*;
use crate::types::board::*;
use crate::types::color::*;
use crate::types::moov::*;
use crate::types::move_list::*;
use crate::types::square::*;

pub type SortScore = i16;

const N_KILLER: usize = 3;

#[derive(Clone)]
pub struct MoveSorter {
    killer_moves: [[[Option<Move>; N_KILLER]; 256]; N_COLORS],
    history_scores: [[SortScore; N_SQUARES]; N_SQUARES],
}

impl MoveSorter {
    pub fn new() -> Self {
        Self {
            killer_moves: [[[None; N_KILLER]; 256]; N_COLORS],
            history_scores: [[0; N_SQUARES]; N_SQUARES],
        }
    }

    pub fn score_moves(
        &self,
        moves: &mut MoveList,
        board: &Board,
        ply: Ply,
        hash_move: &Option<Move>,
    ) {
        let mut m: &mut Move;
        let all_pieces = board.all_pieces();
        for idx in 0..moves.len() {
            m = &mut moves[idx];

            if let Some(hash_move) = hash_move {
                if m == hash_move {
                    m.add_to_score(Self::HASH_MOVE_SCORE);
                    continue;
                }
            }

            if m.is_quiet() {
                if self.is_killer(board, m, ply) {
                    m.add_to_score(Self::KILLER_MOVE_SCORE);
                    continue;
                }

                if m.is_castling() {
                    m.add_to_score(Self::CASTLING_SCORE);
                    continue;
                }

                m.add_to_score(Self::HISTORY_MOVE_OFFSET + self.history_score(m));
                continue;
            }

            if m.is_capture() {
                if m.flags() == MoveFlags::EnPassant {
                    m.add_to_score(Self::WINNING_CAPTURES_OFFSET);
                    continue;
                }

                let capture_value = see(board, m, all_pieces);

                if capture_value >= 0 {
                    m.add_to_score(capture_value + Self::WINNING_CAPTURES_OFFSET);
                } else {
                    m.add_to_score(capture_value + Self::LOSING_CAPTURES_OFFSET);
                }
            }

            if m.is_promotion() {
                match m.flags() {
                    MoveFlags::PcBishop | MoveFlags::PrBishop => {
                        m.add_to_score(Self::BISHOP_PROMOTION_SCORE);
                    }
                    MoveFlags::PcKnight | MoveFlags::PrKnight => {
                        m.add_to_score(Self::KNIGHT_PROMOTION_SCORE);
                    }
                    MoveFlags::PcRook | MoveFlags::PrRook => {
                        m.add_to_score(Self::ROOK_PROMOTION_SCORE);
                    }
                    MoveFlags::PcQueen | MoveFlags::PrQueen => {
                        m.add_to_score(Self::QUEEN_PROMOTION_SCORE);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn add_killer(&mut self, board: &Board, m: Move, ply: Ply) {
        let color = board.color_to_play() as usize;
        self.killer_moves[color][ply].rotate_right(1);
        self.killer_moves[color][ply][0] = Some(m);
    }

    pub fn add_history(&mut self, m: Move, depth: Depth) {
        debug_assert!(depth >= 0, "Depth is less than 0 in the history heuristic!");

        let depth = depth as SortScore;
        let from = m.from_sq().index();
        let to = m.to_sq().index();
        self.history_scores[from][to] += depth * depth;

        if self.history_scores[from][to] >= -Self::HISTORY_MOVE_OFFSET {
            for sq1 in BitBoard::ALL {
                for sq2 in BitBoard::ALL {
                    self.history_scores[sq1.index()][sq2.index()] >>= 1; // Divide by two
                }
            }
        }
    }

    fn is_killer(&self, board: &Board, m: &Move, ply: usize) -> bool {
        let color = board.color_to_play().index();
        for i in 0..self.killer_moves[color][ply].len() {
            if Some(*m) == self.killer_moves[color][ply][i] {
                return true;
            }
        }
        false
    }

    #[inline(always)]
    fn history_score(&self, m: &Move) -> SortScore {
        self.history_scores[m.from_sq().index()][m.to_sq().index()]
    }

    pub fn clear_history(&mut self) {
        for sq1 in BitBoard::ALL {
            for sq2 in BitBoard::ALL {
                self.history_scores[sq1.index()][sq2.index()] = 0;
            }
        }
    }

    pub fn clear_killers(&mut self) {
        for ply in 0..self.killer_moves[0].len() {
            for killer_idx in 0..self.killer_moves[0][0].len() {
                self.killer_moves[Color::White.index()][ply][killer_idx] = None;
                self.killer_moves[Color::Black.index()][ply][killer_idx] = None;
            }
        }
    }
}

impl MoveSorter {
    const HASH_MOVE_SCORE: SortScore = 25000;
    const WINNING_CAPTURES_OFFSET: SortScore = 10;
    const QUEEN_PROMOTION_SCORE: SortScore = 8;
    const ROOK_PROMOTION_SCORE: SortScore = 7;
    const BISHOP_PROMOTION_SCORE: SortScore = 6;
    const KNIGHT_PROMOTION_SCORE: SortScore = 5;
    const KILLER_MOVE_SCORE: SortScore = 2;
    const CASTLING_SCORE: SortScore = 1;
    const HISTORY_MOVE_OFFSET: SortScore = -30000;
    const LOSING_CAPTURES_OFFSET: SortScore = -30001;
}
