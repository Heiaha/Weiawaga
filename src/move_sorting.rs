use super::attacks;
use super::bitboard::*;
use super::board::*;
use super::moov::*;
use super::move_list::*;
use super::piece::*;
use super::square::*;
use super::types::*;

use arrayvec::ArrayVec;

pub struct MoveSorter<'a> {
    moves: &'a mut MoveList,
    scores: ArrayVec<i32, MAX_MOVES>,
    idx: usize,
}

impl Iterator for MoveSorter<'_> {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.moves.len() {
            return None;
        }

        let max_idx = (self.idx..self.moves.len()).max_by_key(|&i| self.scores[i])?;
        self.moves.swap(self.idx, max_idx);
        self.scores.swap(self.idx, max_idx);

        let next_move = self.moves.get(self.idx).copied();
        self.idx += 1;
        next_move
    }
}

pub struct MoveScorer {
    killer_moves: [Option<Move>; MAX_MOVES],
    history_scores: ColorMap<SQMap<SQMap<i32>>>,
    counter_moves: SQMap<SQMap<Option<Move>>>,
}

impl MoveScorer {
    pub fn new() -> Self {
        Self {
            killer_moves: [None; MAX_MOVES],
            history_scores: ColorMap::new(
                [SQMap::new([SQMap::new([0; SQ::N_SQUARES]); SQ::N_SQUARES]); 2],
            ),
            counter_moves: SQMap::new([SQMap::new([None; SQ::N_SQUARES]); SQ::N_SQUARES]),
        }
    }

    pub fn create_sorter<'a, const QUIESCENCE: bool>(
        &self,
        moves: &'a mut MoveList,
        board: &Board,
        ply: usize,
        hash_move: Option<Move>,
    ) -> MoveSorter<'a> {
        MoveSorter {
            scores: ArrayVec::from_iter(
                moves
                    .into_iter()
                    .map(|&m| self.score_move::<QUIESCENCE>(m, board, ply, hash_move)),
            ),
            idx: 0,
            moves,
        }
    }

    fn score_move<const QUIESCENCE: bool>(
        &self,
        m: Move,
        board: &Board,
        ply: usize,
        hash_move: Option<Move>,
    ) -> i32 {
        if Some(m) == hash_move {
            return Self::HASH_MOVE_SCORE;
        }

        if !QUIESCENCE && m.is_quiet() {
            return if self.is_killer(m, ply) {
                Self::KILLER_MOVE_SCORE
            } else if self.is_counter(board, m) {
                Self::COUNTER_MOVE_SCORE
            } else if m.is_castling() {
                Self::CASTLING_SCORE
            } else {
                self.history_score(m, board.ctm())
            };
        }

        let mut score = 0;
        if m.is_capture() {
            if m.is_ep() {
                return Self::CAPTURE_SCORE;
            }

            score += Self::mvv_lva_score(board, m)
                + if QUIESCENCE || Self::see(board, m) {
                    Self::CAPTURE_SCORE
                } else {
                    -Self::CAPTURE_SCORE
                };
        }

        score += m
            .promotion()
            .map_or(0, |pt| Self::PROMOTION_SCORE + Self::SEE_PIECE_TYPE[pt]);
        score
    }

    fn mvv_lva_score(board: &Board, m: Move) -> i32 {
        let (from_sq, to_sq) = m.squares();
        let captured_pt = board.piece_type_at(to_sq).expect("No captured in MVVLVA.");
        let attacking_pt = board
            .piece_type_at(from_sq)
            .expect("No attacker in MVVLVA.");

        Self::MVV_LVA_SCORES[captured_pt.index() * PieceType::N_PIECE_TYPES + attacking_pt.index()]
    }

    pub fn add_killer(&mut self, m: Move, ply: usize) {
        self.killer_moves[ply] = Some(m);
    }

    pub fn add_history(&mut self, m: Move, ctm: Color, depth: i8) {
        let depth = depth as i32;
        let (from_sq, to_sq) = m.squares();
        let score = &mut self.history_scores[ctm][from_sq][to_sq];
        *score += depth * depth;

        if *score >= Self::HISTORY_MAX {
            self.history_scores
                .iter_mut()
                .flatten()
                .flatten()
                .for_each(|x| *x >>= 1);
        }
    }

    pub fn add_counter(&mut self, p_move: Move, m: Move) {
        self.counter_moves[p_move.from_sq()][p_move.to_sq()] = Some(m);
    }

    fn is_killer(&self, m: Move, ply: usize) -> bool {
        self.killer_moves[ply] == Some(m)
    }

    fn is_counter(&self, board: &Board, m: Move) -> bool {
        board
            .peek()
            .is_some_and(|p_move| self.counter_moves[p_move.from_sq()][p_move.to_sq()] == Some(m))
    }

    fn history_score(&self, m: Move, ctm: Color) -> i32 {
        self.history_scores[ctm][m.from_sq()][m.to_sq()]
    }

    pub fn see(board: &Board, m: Move) -> bool {
        if m.promotion().is_some() {
            return true;
        }

        let (from_sq, to_sq) = m.squares();

        let Some(captured_pt) = board.piece_type_at(to_sq) else {
            return true;
        };

        let mut attacking_pt = board
            .piece_type_at(from_sq)
            .expect("No attacking pt in see.");

        let mut value = Self::SEE_PIECE_TYPE[captured_pt] - Self::SEE_PIECE_TYPE[attacking_pt];

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

impl MoveScorer {
    const HISTORY_MAX: i32 = i16::MAX as i32 / 2;
    const HASH_MOVE_SCORE: i32 = 100 * Self::HISTORY_MAX;
    const PROMOTION_SCORE: i32 = 50 * Self::HISTORY_MAX;
    const CAPTURE_SCORE: i32 = 10 * Self::HISTORY_MAX;
    const KILLER_MOVE_SCORE: i32 = 5 * Self::HISTORY_MAX;
    const COUNTER_MOVE_SCORE: i32 = 3 * Self::HISTORY_MAX;
    const CASTLING_SCORE: i32 = 2 * Self::HISTORY_MAX;

    const SEE_PIECE_TYPE: PieceTypeMap<i32> = PieceTypeMap::new([100, 375, 375, 500, 1025, 10000]);

    #[rustfmt::skip]
    const MVV_LVA_SCORES: [i32; PieceType::N_PIECE_TYPES * PieceType::N_PIECE_TYPES] = [
        105, 104, 103, 102, 101, 100,
        205, 204, 203, 202, 201, 200,
        305, 304, 303, 302, 301, 300,
        405, 404, 403, 402, 401, 400,
        505, 504, 503, 502, 501, 500,
        605, 604, 603, 602, 601, 600
    ];
}
