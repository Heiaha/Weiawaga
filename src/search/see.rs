use super::move_sorter::*;
use crate::types::attacks;
use crate::types::bitboard::*;
use crate::types::board::*;
use crate::types::moov::*;
use crate::types::piece::*;
use std::cmp::max;

const SEE_PIECE_TYPE: [SortScore; 6] = [100, 375, 375, 500, 1025, 10000];

// https://www.chessprogramming.org/SEE_-_The_Swap_Algorithm
// Implementation adapted from Black Marlin
pub fn see(board: &Board, m: &Move, all_pieces: BitBoard) -> SortScore {
    let mut max_depth = 0;
    let mut defenders;
    let mut piece_bb;

    let to_sq = m.to_sq();
    let mut gains = [0; 16];
    let mut color = !board.color_to_play();
    let mut blockers = all_pieces & !m.from_sq().bb();

    gains[0] = SEE_PIECE_TYPE[board.piece_type_at(to_sq).index()];
    let mut last_piece_pts = SEE_PIECE_TYPE[board.piece_type_at(m.from_sq()).index()];

    'depth_loop: for depth in 1..gains.len() {
        gains[depth] = last_piece_pts - gains[depth - 1];
        defenders = board.all_pieces_color(color) & blockers;
        for pt in PieceType::iter(PieceType::Pawn, PieceType::King) {
            last_piece_pts = SEE_PIECE_TYPE[pt.index()];
            piece_bb = if pt == PieceType::Pawn {
                attacks::pawn_attacks_sq(to_sq, !color)
                    & defenders
                    & board.bitboard_of_piecetype(PieceType::Pawn)
            } else {
                attacks::attacks(pt, to_sq, blockers) & defenders & board.bitboard_of_piecetype(pt)
            };
            if piece_bb != BitBoard::ZERO {
                blockers &= !piece_bb.lsb().bb();
                color = !color;
                continue 'depth_loop;
            }
        }
        max_depth = depth;
        break;
    }

    for depth in (1..max_depth).rev() {
        gains[depth - 1] = -max(-gains[depth - 1], gains[depth]);
    }
    gains[0]
}
