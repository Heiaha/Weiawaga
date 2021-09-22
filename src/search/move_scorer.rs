use crate::search::search::{Depth, Ply};
use crate::types::board::Board;
use crate::types::color::N_COLORS;
use crate::types::moov::{Move, MoveFlags};
use crate::types::move_list::MoveList;
use crate::types::piece::PieceType;
use crate::types::square::{N_SQUARES, SQ};
use std::cmp::min;

pub type SortScore = u16;

static mut MVV_LVA_SCORES: [[SortScore; 6]; 6] = [[0; 6]; 6];
const N_KILLER: usize = 1;

pub struct MoveScorer {
    killer_moves: [[[Move; N_KILLER]; 1000]; N_COLORS],
    history_scores: [[SortScore; N_SQUARES]; N_SQUARES],
}

impl MoveScorer {
    pub fn new() -> MoveScorer {
        MoveScorer {
            killer_moves: [[[Move::NULL; N_KILLER]; 1000]; N_COLORS],
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
        if moves.len() == 0 {
            return;
        }

        let mut m: &mut Move;

        for i in 0..moves.len() {
            m = &mut moves[i];

            if let Some(hash_move) = hash_move {
                if m == hash_move {
                    m.add_to_score(Self::HASH_MOVE_SCORE);
                }
            }

            if self.is_killer(board, m, ply) {
                m.add_to_score(Self::KILLER_MOVE_SCORE);
            }

            match m.flags() {
                MoveFlags::PcBishop
                | MoveFlags::PcKnight
                | MoveFlags::PcRook
                | MoveFlags::PcQueen => {
                    m.add_to_score(Self::PROMOTION_SCORE);
                    m.add_to_score(Self::CAPTURE_SCORE);
                    m.add_to_score(Self::mvv_lva_score(board, m));
                }
                MoveFlags::PrBishop
                | MoveFlags::PrKnight
                | MoveFlags::PrRook
                | MoveFlags::PrQueen => {
                    m.add_to_score(Self::PROMOTION_SCORE);
                }
                MoveFlags::Capture => {
                    m.add_to_score(Self::CAPTURE_SCORE);
                    m.add_to_score(Self::mvv_lva_score(board, m));
                }
                _ => m.add_to_score(min(Self::KILLER_MOVE_SCORE, self.history_score(m))),
            }
        }
    }

    pub fn add_killer(&mut self, board: &Board, m: Move, ply: Ply) {
        let color = board.color_to_play() as usize;
        self.killer_moves[color][ply].rotate_right(1);
        self.killer_moves[color][ply][0] = m;
    }

    pub fn add_history(&mut self, m: Move, depth: Depth) {
        debug_assert!(depth >= 0, "Depth is less than 0 in the history heuristic!");

        let depth = depth as u16;
        let from = m.from_sq().index();
        let to = m.to_sq().index();
        self.history_scores[from][to] += depth * depth;

        if self.history_scores[from][to] > u16::MAX / 2 {
            for sq1 in SQ::A1..=SQ::H8 {
                for sq2 in SQ::A1..=SQ::H8 {
                    self.history_scores[sq1.index()][sq2.index()] /= 2;
                }
            }
        }
    }

    fn is_killer(&self, board: &Board, m: &Move, ply: usize) -> bool {
        let color = board.color_to_play().index();
        for i in 0..self.killer_moves[color][ply].len() {
            if *m == self.killer_moves[color][ply][i] {
                return true;
            }
        }
        false
    }

    #[inline(always)]
    fn history_score(&self, m: &Move) -> SortScore {
        self.history_scores[m.from_sq().index()][m.to_sq().index()]
    }

    #[inline(always)]
    fn mvv_lva_score(board: &Board, m: &Move) -> SortScore {
        unsafe {
            MVV_LVA_SCORES[board.piece_type_at(m.to_sq()).index()]
                [board.piece_type_at(m.from_sq()).index()]
        }
    }
}

impl MoveScorer {
    const HASH_MOVE_SCORE: SortScore = 10000;
    const PROMOTION_SCORE: SortScore = 5000;
    const CAPTURE_SCORE: SortScore = 200;
    const KILLER_MOVE_SCORE: SortScore = 90;
}

//////////////////////////////////////////////
// Init
//////////////////////////////////////////////

fn init_mvv_lva(mvv_lva_scores: &mut [[u16; 6]; 6]) {
    let victim_score: [SortScore; 6] = [100, 200, 300, 400, 500, 600];
    for attacker in PieceType::Pawn..=PieceType::King {
        for victim in PieceType::Pawn..=PieceType::King {
            mvv_lva_scores[victim.index()][attacker.index()] =
                victim_score[victim.index()] + 6 - (victim_score[attacker.index()] / 100);
        }
    }
}

pub fn init_move_orderer() {
    unsafe {
        init_mvv_lva(&mut MVV_LVA_SCORES);
    }
}
