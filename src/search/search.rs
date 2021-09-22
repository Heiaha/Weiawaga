use crate::types::moov::{Move, MoveFlags};
use super::timer::*;
use crate::types::board::Board;
use crate::evaluation::score::{Score, Value};
use crate::types::move_list::MoveList;
use crate::types::bitboard::BitBoard;
use crate::search::move_scorer::MoveScorer;
use crate::evaluation::eval::eval;
use crate::types::piece::PieceType;
use crate::types::color::Color;
use std::cmp::{min, max};
use crate::search::tt::{TT, TTFlag};
use crate::search::statistics::Statistics;

pub type Depth = i8;
pub type Ply = usize;

pub struct Search<'a> {
    stop: bool,
    sel_depth: Ply,
    timer: Timer,
    sorter: MoveScorer,
    tt: &'a mut TT,
    stats: Statistics,
}

impl<'a> Search<'a> {
    pub fn new(timer: Timer, tt: &'a mut TT) -> Self {
        Search {
            stop: false,
            sel_depth: 0,
            timer: timer,
            sorter: MoveScorer::new(),
            tt: tt,
            stats: Statistics::new(),
        }
    }

    pub fn go(&mut self, board: &mut Board) -> (Move, Value) {
        let mut alpha = -Score::INF;
        let mut beta = Score::INF;
        let mut depth = 1;
        let mut final_move = Move::null();
        let mut final_score = 0;
        let mut last_score = 0;

        let mut moves = MoveList::new();
        board.generate_legal_moves(&mut moves);
        if moves.len() == 1 {
            return (moves[0], 0);
        }

        while !self.stop && self.timer.start_check(depth) && !Score::is_checkmate(final_score) {
            (final_move, final_score) = self.negamax_root(board, depth, alpha, beta);

            if depth >= 4 {
                self.timer.update(final_score - last_score);
            }
            last_score = final_score;

            if final_score <= alpha {
                alpha = -Score::INF;
            }
            else if final_score >= beta {
                beta = Score::INF;
            }
            else {
                self.print_info(board, depth, final_move, final_score);
                alpha = final_score - Self::ASPIRATION_WINDOW;
                beta = final_score + Self::ASPIRATION_WINDOW;
                depth += 1;
                self.stats = Statistics::new();
            }
        }
        (final_move, final_score)
    }

    pub fn negamax_root(&mut self, board: &mut Board, mut depth: Depth, mut alpha: Value, mut beta: Value) -> (Move, Value) {
        let mut moves = MoveList::new();
        board.generate_legal_moves(&mut moves);

        let in_check = board.checkers() != BitBoard::ZERO;
        if in_check {
            depth += 1;
        }

        let mut best_move: Move = Move::null();
        if moves.len() == 1 {
            best_move = moves[0];
            return (best_move, 0);
        }

        let mut hash_move = None;
        if let Some(tt_entry) = self.tt.probe(board.hash()) {
            hash_move = tt_entry.best_move();
        }

        let mut value: Value;
        self.sorter.score_moves(&mut moves, &board, 0, &hash_move);
        while let Some(m) = moves.next_best() {

            board.push(m);
            value = -self.negamax(board, depth - 1, 1, -beta, -alpha, true);
            board.pop();

            if self.stop || self.timer.stop_check() {
                self.stop = true;
                break;
            }

            if value > alpha {
                best_move = m;
                if value >= beta {
                    self.tt.insert(board.hash(), depth, beta, Some(best_move), TTFlag::LOWER);
                    return (best_move, beta);
                }
                alpha = value;
                self.tt.insert(board.hash(), depth, alpha, Some(best_move), TTFlag::UPPER);
            }
        }

        if best_move == Move::null() {
            best_move = moves[0];
        }

        if !self.stop {
            self.tt.insert(board.hash(), depth, alpha, Some(best_move), TTFlag::EXACT);
        }
        (best_move, alpha)
    }

    fn negamax(&mut self, board: &mut Board, depth: Depth, ply: Ply, mut alpha: Value, mut beta: Value, can_apply_null: bool) -> Value {
        if self.stop || self.timer.stop_check() {
            self.stop = true;
            return 0;
        }

        let mate_value = Score::INF - (ply as Value);
        if alpha < -mate_value { alpha = -mate_value; }
        if beta > mate_value - 1 { beta = mate_value - 1; }
        if alpha >= beta {
            self.stats.leafs += 1;
            return alpha;
        }

        let in_check = board.king_attacked();
        if depth <= 0 && !in_check { return self.q_search(board, ply, alpha, beta) }
        self.stats.nodes += 1;

        if board.is_repetition_or_fifty() {
            self.stats.leafs += 1;
            println!("draw");
            return 0;
        }

        let mut hash_move: Option<Move> = None;
        if let Some(tt_entry) = self.tt.probe(board.hash()) {
            if tt_entry.depth() >= depth {
                self.stats.tt_hits += 1;
                match tt_entry.flag() {
                    TTFlag::EXACT => {
                        self.stats.leafs += 1;
                        return tt_entry.value();
                    }
                    TTFlag::LOWER => { alpha = max(alpha, tt_entry.value()); }
                    TTFlag::UPPER => { beta = min(beta, tt_entry.value()); }
                }
                if alpha >= beta {
                    self.stats.leafs += 1;
                    self.stats.beta_cutoffs += 1;
                    return tt_entry.value();
                }
            }
            hash_move = tt_entry.best_move();
        }

        if Self::can_apply_null(board, depth, beta, in_check, can_apply_null) {
            let r = if depth > 6 { 3 } else { 2 };
            board.push_null();
            let value = -self.negamax(board, depth - r - 1, ply, -beta, -beta + 1, false);
            board.pop_null();
            if self.stop {
                return 0;
            }
            if value >= beta {
                self.stats.beta_cutoffs += 1;
                return beta;
            }
        }

        let mut value: Value;
        let mut reduced_depth: Depth;
        let mut best_move: Option<Move> = None;
        let mut tt_flag = TTFlag::UPPER;
        let mut moves = MoveList::new();
        let mut idx = 0;

        board.generate_legal_moves(&mut moves);
        self.sorter.score_moves(&mut moves, &board, ply, &hash_move);
        while let Some(m) = moves.next_best() {

            reduced_depth = depth;
            if Self::can_apply_lmr(&m, depth, idx) {
                reduced_depth -= Self::late_move_reduction(depth, idx);
            }

            if in_check {
                reduced_depth += 1;
            }

            board.push(m);
            value = -self.negamax(board, reduced_depth - 1, ply + 1, -beta, -alpha, true);
            board.pop();

            if self.stop {
                return 0;
            }

            if value > alpha {
                best_move = Some(m);
                if value >= beta {
                    if m.flags() == MoveFlags::Quiet {
                        self.sorter.add_killer(&board, m, ply);
                        self.sorter.add_history(m, depth);
                    }
                    self.stats.beta_cutoffs += 1;
                    tt_flag = TTFlag::LOWER;
                    alpha = beta;
                    break;
                }
                tt_flag = TTFlag::EXACT;
                alpha = value;
            }
            idx += 1;
        }

        if moves.len() == 0 {
            if in_check {
                alpha = -mate_value;
            }
            else {
                alpha = 0;
            }
        }

        if !self.stop {
            self.tt.insert(board.hash(), depth, alpha, best_move, tt_flag);
        }
        alpha
    }

    fn q_search(&mut self, board: &mut Board, ply: Ply, mut alpha: Value, beta: Value) -> Value {
        if self.stop || self.timer.stop_check() {
            self.stop = true;
            return 0;
        }

        self.sel_depth = max(self.sel_depth, ply);
        self.stats.qnodes += 1;

        let value = eval(board);

        if value >= beta {
            self.stats.qleafs += 1;
            return beta;
        }

        if alpha < value {
            alpha = value;
        }

        let mut hash_move: Option<Move> = None;
        if let Some(tt_entry) = self.tt.probe(board.hash()) {
            hash_move = tt_entry.best_move();
        }

        let mut value= 0;

        let mut moves = MoveList::new();
        board.generate_legal_q_moves(&mut moves);
        self.sorter.score_moves(&mut moves, &board, ply, &hash_move);
        while let Some(m) = moves.next_best() {

            board.push(m);
            value = -self.q_search(board, ply + 1, -beta, -alpha);
            board.pop();

            if self.stop {
                return 0;
            }

            if value > alpha {
                if value >= beta {
                    self.stats.qbeta_cutoffs += 1;
                    return beta;
                }
                alpha = value;
            }
        }
        alpha
    }

    #[inline(always)]
    fn can_apply_null(board: &Board, depth: Depth, beta: Value, in_check: bool, can_apply_null: bool) -> bool {
        can_apply_null &&
            !in_check &&
            depth >= Self::NULL_MIN_DEPTH &&
            board.has_non_pawn_material() &&
            eval(&board) >= beta
    }

    #[inline(always)]
    fn can_apply_lmr(m: &Move, depth: Depth, move_index: usize) -> bool {
        depth > Self::LMR_MIN_DEPTH &&
            move_index > Self::LMR_MOVE_WO_REDUCTION &&
            m.flags() == MoveFlags::Quiet
    }

    #[inline(always)]
    fn late_move_reduction(depth: Depth, move_index: usize) -> Depth {
        unsafe {
            LMR_TABLE[min(depth as usize, 63)][min(move_index, 63)]
        }
    }

    fn get_pv(&self, board: &mut Board, depth: Depth) -> String {
        if depth == 0 {
            return "".to_owned();
        }
        let mut hash_move: Option<Move> = None;
        let tt_entry = self.tt.probe(board.hash());
        match tt_entry {
            Some(tt_entry) => {
                hash_move = tt_entry.best_move();
                if hash_move == None {
                    return "".to_owned();
                }
            }
            None => { return "".to_owned(); }
        }

        board.push(hash_move.unwrap());
        let pv = hash_move.unwrap().to_string() + " " + &*self.get_pv(board, depth - 1);
        board.pop();

        pv
    }

    fn print_info(&self, board: &mut Board, depth: Depth, m: Move, score: Value) {
        println!("info currmove {move} depth {depth} seldepth {sel_depth} time {time} score cp {score} nodes {nodes} nps {nps} pv {pv}",
                 move=m.to_string(),
                 depth=depth,
                 sel_depth=self.sel_depth,
                 time=self.timer.elapsed(),
                 score=score,
                 nodes=self.stats.total_nodes(),
                 nps=1000*self.stats.total_nodes()/(self.timer.elapsed() + 1),
                 pv=self.get_pv(board, depth)
        );
    }

}



impl<'a> Search<'a> {
    const ASPIRATION_WINDOW: Value = 25;
    const NULL_MIN_DEPTH: Depth = 2;
    const LMR_MOVE_WO_REDUCTION: usize = 1;
    const LMR_MIN_DEPTH: Depth = 2;
}

pub static mut LMR_TABLE: [[Depth; 64]; 64] = [[0; 64]; 64];



fn init_lmr_table(lmr_table: &mut [[Depth; 64]; 64]) {
    for depth in 1..64 {
        for move_number in 1..64 {
            lmr_table[depth][move_number] = (0.75_f32 + f32::ln(depth as f32) * f32::ln(move_number as f32)/2.25_f32) as Depth;

        }
    }
}

pub fn init_search() {
    unsafe {
        init_lmr_table(&mut LMR_TABLE);
    }
}