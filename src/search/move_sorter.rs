use super::search::*;
use crate::types::board::*;
use crate::types::color::*;
use crate::types::moov::*;
use crate::types::move_list::*;
use crate::types::piece::*;
use crate::types::square::*;
use std::cmp::min;
use std::ops::{Index, IndexMut};

pub type SortScore = u16;

static mut MVV_LVA_SCORES: [[SortScore; 6]; 6] = [[0; 6]; 6];
const N_KILLER: usize = 3;
const HISTORY_MAX: SortScore = SortScore::MAX / 2;

static mut KILLER_MOVES: [[[Option<Move>; N_KILLER]; 256]; N_COLORS] =
    [[[None; N_KILLER]; 256]; N_COLORS];
static mut HISTORY_SCORES: [[SortScore; N_SQUARES]; N_SQUARES] = [[0; N_SQUARES]; N_SQUARES];

pub struct MoveSorter(MoveList);

impl MoveSorter {
    pub fn new(board: &mut Board, ply: Ply, hash_move: &Option<Move>) -> MoveSorter {
        let mut moves = MoveList::new();
        board.generate_legal_moves(&mut moves);
        MoveSorter::score_moves(&mut moves, board, ply, hash_move);
        MoveSorter(moves)
    }

    pub fn new_q(board: &mut Board, ply: Ply, hash_move: &Option<Move>) -> MoveSorter {
        let mut moves = MoveList::new();
        board.generate_legal_q_moves(&mut moves);
        MoveSorter::score_moves(&mut moves, board, ply, hash_move);
        MoveSorter(moves)
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    fn score_moves(moves: &mut MoveList, board: &Board, ply: Ply, hash_move: &Option<Move>) {
        let mut m: &mut Move;
        for idx in 0..moves.len() {
            m = &mut moves[idx];

            if let Some(hash_move) = hash_move {
                if m == hash_move {
                    m.add_to_score(Self::HASH_MOVE_SCORE);
                }
            }

            if Self::is_killer(board, m, ply) {
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
                _ => m.add_to_score(min(Self::KILLER_MOVE_SCORE, Self::history_score(m))),
            }
        }
    }

    pub fn add_killer(board: &Board, m: Move, ply: Ply) {
        let color = board.color_to_play() as usize;
        unsafe {
            KILLER_MOVES[color][ply].rotate_right(1);
            KILLER_MOVES[color][ply][0] = Some(m);
        }
    }

    pub fn add_history(m: Move, depth: Depth) {
        debug_assert!(depth >= 0, "Depth is less than 0 in the history heuristic!");

        let depth = depth as SortScore;
        let from = m.from_sq().index();
        let to = m.to_sq().index();
        unsafe {
            HISTORY_SCORES[from][to] += depth * depth;

            if HISTORY_SCORES[from][to] > HISTORY_MAX {
                for sq1 in SQ::A1..=SQ::H8 {
                    for sq2 in SQ::A1..=SQ::H8 {
                        HISTORY_SCORES[sq1.index()][sq2.index()] >>= 1; // Divide by two
                    }
                }
            }
        }
    }

    fn is_killer(board: &Board, m: &Move, ply: usize) -> bool {
        let color = board.color_to_play().index();
        unsafe {
            for i in 0..KILLER_MOVES[color][ply].len() {
                if Some(*m) == KILLER_MOVES[color][ply][i] {
                    return true;
                }
            }
        }
        false
    }

    #[inline(always)]
    fn history_score(m: &Move) -> SortScore {
        unsafe { HISTORY_SCORES[m.from_sq().index()][m.to_sq().index()] }
    }

    #[inline(always)]
    fn mvv_lva_score(board: &Board, m: &Move) -> SortScore {
        unsafe {
            MVV_LVA_SCORES[board.piece_type_at(m.to_sq()).index()]
                [board.piece_type_at(m.from_sq()).index()]
        }
    }

    pub fn clear_history() {
        for sq1 in SQ::A1..=SQ::H8 {
            for sq2 in SQ::A1..=SQ::H8 {
                unsafe {
                    HISTORY_SCORES[sq1.index()][sq2.index()] = 0;
                }
            }
        }
    }

    pub fn clear_killers() {
        unsafe {
            for ply in 0..KILLER_MOVES[0].len() {
                for killer_idx in 0..KILLER_MOVES[0][0].len() {
                    KILLER_MOVES[Color::White.index()][ply][killer_idx] = None;
                    KILLER_MOVES[Color::Black.index()][ply][killer_idx] = None;
                }
            }
        }
    }
}

impl Iterator for MoveSorter {
    type Item = Move;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_best()
    }
}

impl Index<usize> for MoveSorter {
    type Output = Move;

    #[inline(always)]
    fn index(&self, i: usize) -> &Self::Output {
        &self.0[i]
    }
}

impl IndexMut<usize> for MoveSorter {
    #[inline(always)]
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        &mut self.0[i]
    }
}

impl MoveSorter {
    const HASH_MOVE_SCORE: SortScore = 10000;
    const PROMOTION_SCORE: SortScore = 5000;
    const CAPTURE_SCORE: SortScore = 200;
    const KILLER_MOVE_SCORE: SortScore = 90;
}

//////////////////////////////////////////////
// Init
//////////////////////////////////////////////

fn init_mvv_lva(mvv_lva_scores: &mut [[SortScore; 6]; 6]) {
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
