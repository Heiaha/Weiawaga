use super::search::*;
use super::see::*;
use crate::types::bitboard::BitBoard;
use crate::types::board::*;
use crate::types::color::*;
use crate::types::moov::*;
use crate::types::move_list::*;
use crate::types::square::*;
use std::ops::IndexMut;

pub type SortScore = i16;

const N_KILLER: usize = 3;

static mut KILLER_MOVES: [[[Option<Move>; N_KILLER]; 256]; N_COLORS] =
    [[[None; N_KILLER]; 256]; N_COLORS];
static mut HISTORY_SCORES: [[SortScore; N_SQUARES]; N_SQUARES] = [[0; N_SQUARES]; N_SQUARES];

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

pub fn score_moves(moves: &mut MoveList, board: &Board, ply: Ply, hash_move: &Option<Move>) {
    let mut m: &mut Move;
    let all_pieces = board.all_pieces();
    for idx in 0..moves.len() {
        m = &mut moves[idx];

        if let Some(hash_move) = hash_move {
            if m == hash_move {
                m.add_to_score(HASH_MOVE_SCORE);
                continue;
            }
        }

        if m.is_quiet() {
            if is_killer(board, m, ply) {
                m.add_to_score(KILLER_MOVE_SCORE);
                continue;
            }

            if m.is_castling() {
                m.add_to_score(CASTLING_SCORE);
                continue;
            }

            m.add_to_score(HISTORY_MOVE_OFFSET + history_score(m));
            continue;
        }

        if m.is_capture() {
            if m.flags() == MoveFlags::EnPassant {
                m.add_to_score(WINNING_CAPTURES_OFFSET);
                continue;
            }

            let capture_value = see(board, m, all_pieces);

            if capture_value >= 0 {
                m.add_to_score(capture_value + WINNING_CAPTURES_OFFSET);
            } else {
                m.add_to_score(capture_value + LOSING_CAPTURES_OFFSET);
            }
        }

        if m.is_promotion() {
            match m.flags() {
                MoveFlags::PcBishop | MoveFlags::PrBishop => {
                    m.add_to_score(BISHOP_PROMOTION_SCORE);
                }
                MoveFlags::PcKnight | MoveFlags::PrKnight => {
                    m.add_to_score(KNIGHT_PROMOTION_SCORE);
                }
                MoveFlags::PcRook | MoveFlags::PrRook => {
                    m.add_to_score(ROOK_PROMOTION_SCORE);
                }
                MoveFlags::PcQueen | MoveFlags::PrQueen => {
                    m.add_to_score(QUEEN_PROMOTION_SCORE);
                }
                _ => {}
            }
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

        if HISTORY_SCORES[from][to] >= -HISTORY_MOVE_OFFSET {
            for sq1 in BitBoard::ALL {
                for sq2 in BitBoard::ALL {
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

pub fn clear_history() {
    for sq1 in BitBoard::ALL {
        for sq2 in BitBoard::ALL {
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
