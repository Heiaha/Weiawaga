use std::cmp::{max, min};

use super::board::*;
use super::moov::*;
use super::move_list::*;
use super::move_sorter::*;
use super::timer::*;
use super::tt::*;
use super::types::*;

pub struct Search<'a> {
    id: u16,
    stop: bool,
    sel_depth: Ply,
    timer: Timer,
    tt: &'a TT,
    nodes: u64,
    move_sorter: MoveSorter,
}

impl<'a> Search<'a> {
    pub fn new(timer: Timer, tt: &'a TT, id: u16) -> Self {
        Self {
            id,
            timer,
            tt,
            stop: false,
            sel_depth: 0,
            nodes: 0,
            move_sorter: MoveSorter::new(),
        }
    }

    pub fn go(&mut self, mut board: Board) -> Move {
        ///////////////////////////////////////////////////////////////////
        // Start iterative deepening.
        ///////////////////////////////////////////////////////////////////
        let mut alpha = -Value::MAX;
        let mut beta = Value::MAX;
        let mut best_move = None;
        let mut score = 0;
        let mut depth = 1;

        ///////////////////////////////////////////////////////////////////
        // If there's only one legal move, just play
        // it instead of searching.
        ///////////////////////////////////////////////////////////////////
        let moves = MoveList::from(&board);
        if moves.len() == 1 {
            return moves[0];
        }

        while !self.stop
            && self.timer.start_check(depth)
            && !Self::is_checkmate(score)
            && depth < Depth::MAX
        {
            (best_move, score) = self.search_root(&mut board, depth, alpha, beta);

            ///////////////////////////////////////////////////////////////////
            // Update the clock if the score is changing
            // by a lot.
            ///////////////////////////////////////////////////////////////////
            if depth >= Self::SEARCHES_WO_TIMER_UPDATE {
                self.timer.update(score);
            }

            ///////////////////////////////////////////////////////////////////
            // Widen aspiration windows.
            ///////////////////////////////////////////////////////////////////
            if score <= alpha {
                alpha = -Value::MAX;
            } else if score >= beta {
                beta = Value::MAX;
            } else {
                // Only print info if we're in the main thread
                if self.id == 0 && best_move.is_some() {
                    self.print_info(&mut board, depth, best_move, score);
                }
                alpha = score - Self::ASPIRATION_WINDOW;
                beta = score + Self::ASPIRATION_WINDOW;
                depth += 1;
                self.nodes = 0;
                self.sel_depth = 0;
            }
        }

        if self.id == 0 {
            self.timer.stop();
        }

        match best_move {
            Some(m) => m,
            None => moves[0],
        }
    }

    fn search_root(
        &mut self,
        board: &mut Board,
        mut depth: Depth,
        mut alpha: Value,
        beta: Value,
    ) -> (Option<Move>, Value) {
        ///////////////////////////////////////////////////////////////////
        // Check extension.
        ///////////////////////////////////////////////////////////////////
        if board.in_check() {
            depth += 1;
        }

        ///////////////////////////////////////////////////////////////////
        // Check the hash table for the current
        // position, primarily for move ordering.
        ///////////////////////////////////////////////////////////////////
        let mut hash_move = None;
        if let Some(tt_entry) = self.tt.probe(board) {
            hash_move = tt_entry.best_move();
        }

        ///////////////////////////////////////////////////////////////////
        // Score moves and begin searching recursively.
        ///////////////////////////////////////////////////////////////////
        let ply: Ply = 0;
        let mut value: Value = -Value::MAX;
        let mut best_move = None;
        let mut idx = 0;

        let mut moves = MoveList::from(board);
        self.move_sorter
            .score_moves(&mut moves, board, ply, hash_move);

        while let Some(m) = moves.next_best() {
            if self.id == 0 && self.timer.elapsed() >= Self::PRINT_CURRMOVENUMBER_TIME_MILLIS {
                Self::print_currmovenumber(depth, m, idx);
            }

            board.push(m);
            if idx == 0 || -self.search(board, depth - 1, ply + 1, -alpha - 1, -alpha) > alpha {
                value = -self.search(board, depth - 1, ply + 1, -beta, -alpha);
            }
            board.pop();

            if self.stop {
                break;
            }

            if value > alpha {
                best_move = Some(m);
                if value >= beta {
                    self.tt.insert(board, depth, beta, best_move, Bound::Lower);
                    return (best_move, beta);
                }
                alpha = value;
                self.tt.insert(board, depth, alpha, best_move, Bound::Upper);
            }
            idx += 1;
        }

        if best_move.is_none() {
            best_move = Some(moves[0]);
        }

        if !self.stop {
            self.tt.insert(board, depth, alpha, best_move, Bound::Exact);
        }
        (best_move, alpha)
    }

    fn search(
        &mut self,
        board: &mut Board,
        mut depth: Depth,
        ply: Ply,
        mut alpha: Value,
        mut beta: Value,
    ) -> Value {
        if self.stop || self.timer.stop_check() {
            self.stop = true;
            return 0;
        }

        ///////////////////////////////////////////////////////////////////
        // Mate distance pruning - will help reduce
        // some nodes when checkmate is near.
        ///////////////////////////////////////////////////////////////////
        let mate_value = Value::MAX - (ply as Value);
        alpha = max(alpha, -mate_value);
        beta = min(beta, mate_value - 1);
        if alpha >= beta {
            self.nodes += 1;
            return alpha;
        }

        ///////////////////////////////////////////////////////////////////
        // Extend search if position is in check. Check if we're in a pv
        ///////////////////////////////////////////////////////////////////
        let in_check = board.in_check();
        if in_check {
            depth += 1;
        }

        ///////////////////////////////////////////////////////////////////
        // Quiescence search - here we search tactical
        // moves after the main search to prevent a
        // horizon effect.
        ///////////////////////////////////////////////////////////////////
        if depth <= 0 {
            return self.q_search(board, ply, alpha, beta);
        }

        self.nodes += 1;

        if board.is_draw() {
            return 0;
        }

        ///////////////////////////////////////////////////////////////////
        // Probe the hash table and adjust the value.
        // If appropriate, produce a cutoff.
        ///////////////////////////////////////////////////////////////////
        let mut hash_move: Option<Move> = None;
        if let Some(tt_entry) = self.tt.probe(board) {
            if tt_entry.depth() >= depth {
                match tt_entry.flag() {
                    Bound::Exact => return tt_entry.value(),
                    Bound::Lower => alpha = max(alpha, tt_entry.value()),
                    Bound::Upper => beta = min(beta, tt_entry.value()),
                }
                if alpha >= beta {
                    return tt_entry.value();
                }
            }
            hash_move = tt_entry.best_move();
        }

        ///////////////////////////////////////////////////////////////////
        // Check if we're in a pv node
        ///////////////////////////////////////////////////////////////////
        let is_pv = alpha != beta - 1;

        ///////////////////////////////////////////////////////////////////
        // Reverse Futility Pruning
        ///////////////////////////////////////////////////////////////////
        if Self::can_apply_rfp(depth, in_check, is_pv, beta) {
            let eval = board.eval();
            if eval - Self::rfp_margin(depth) >= beta {
                return eval;
            }
        }

        ///////////////////////////////////////////////////////////////////
        // Null move pruning.
        ///////////////////////////////////////////////////////////////////
        if Self::can_apply_null(board, depth, beta, in_check, is_pv) {
            let r = Self::null_reduction(depth);
            board.push_null();
            let value = -self.search(board, depth - r - 1, ply, -beta, -beta + 1);
            board.pop_null();
            if self.stop {
                return 0;
            }
            if value >= beta {
                return beta;
            }
        }

        ///////////////////////////////////////////////////////////////////
        // Generate moves, score, and begin searching
        // recursively.
        ///////////////////////////////////////////////////////////////////
        let mut value: Value;
        let mut reduced_depth: Depth;
        let mut tt_flag = Bound::Upper;
        let mut best_move: Option<Move> = None;
        let mut idx = 0;

        let mut moves = MoveList::from(board);
        self.move_sorter
            .score_moves(&mut moves, board, ply, hash_move);

        while let Some(m) = moves.next_best() {
            ///////////////////////////////////////////////////////////////////
            // Make move and deepen search via principal variation search.
            ///////////////////////////////////////////////////////////////////
            board.push(m);

            if idx == 0 {
                value = -self.search(board, depth - 1, ply + 1, -beta, -alpha);
            } else {
                ///////////////////////////////////////////////////////////////////
                // Late move reductions.
                ///////////////////////////////////////////////////////////////////
                reduced_depth = depth;
                if Self::can_apply_lmr(m, depth, idx) {
                    reduced_depth -= Self::late_move_reduction(depth, idx);
                }
                loop {
                    value = -self.search(board, reduced_depth - 1, ply + 1, -alpha - 1, -alpha);
                    if value > alpha {
                        value = -self.search(board, reduced_depth - 1, ply + 1, -beta, -alpha);
                    }

                    ///////////////////////////////////////////////////////////////////
                    // A reduced depth may bring us above alpha. This is relatively
                    // unusual, but if so we need the exact score so we do a full search.
                    ///////////////////////////////////////////////////////////////////
                    if reduced_depth < depth && value > alpha {
                        reduced_depth = depth;
                    } else {
                        break;
                    }
                }
            }

            board.pop();

            if self.stop {
                return 0;
            }

            ///////////////////////////////////////////////////////////////////
            // Re-bound, check for cutoffs, and add killers and history.
            ///////////////////////////////////////////////////////////////////
            if value > alpha {
                best_move = Some(m);
                if value >= beta {
                    if m.is_quiet() {
                        self.move_sorter.add_killer(board, m, ply);
                        self.move_sorter.add_history(m, depth);
                    }
                    tt_flag = Bound::Lower;
                    alpha = beta;
                    break;
                }
                tt_flag = Bound::Exact;
                alpha = value;
            }
            idx += 1;
        }

        ///////////////////////////////////////////////////////////////////
        // Checkmate and stalemate check.
        ///////////////////////////////////////////////////////////////////
        if moves.len() == 0 {
            if in_check {
                alpha = -mate_value;
            } else {
                alpha = 0;
            }
        }

        if !self.stop {
            self.tt.insert(board, depth, alpha, best_move, tt_flag);
        }
        alpha
    }

    fn q_search(&mut self, board: &mut Board, ply: Ply, mut alpha: Value, beta: Value) -> Value {
        if self.stop || self.timer.stop_check() {
            self.stop = true;
            return 0;
        }

        self.nodes += 1;

        if board.is_draw() {
            return 0;
        }

        self.sel_depth = max(self.sel_depth, ply);

        let mut value = board.eval();

        if value >= beta {
            return beta;
        }
        alpha = max(alpha, value);

        let mut hash_move: Option<Move> = None;
        if let Some(tt_entry) = self.tt.probe(board) {
            hash_move = tt_entry.best_move();
        }

        let mut moves = MoveList::from_q(board);
        self.move_sorter
            .score_moves(&mut moves, board, ply, hash_move);

        let mut idx = 0;
        while let Some(m) = moves.next_best() {
            ///////////////////////////////////////////////////////////////////
            // Effectively a SEE check. Bad captures will have a score < 0
            // given by the SEE + the bad capture offset,
            // and here we skip bad captures.
            ///////////////////////////////////////////////////////////////////
            if moves.scores[idx] < 0 {
                break;
            }

            board.push(m);
            value = -self.q_search(board, ply + 1, -beta, -alpha);
            board.pop();

            if self.stop {
                return 0;
            }

            if value > alpha {
                if value >= beta {
                    return beta;
                }
                alpha = value;
            }
            idx += 1;
        }
        alpha
    }

    #[inline(always)]
    fn can_apply_null(
        board: &Board,
        depth: Depth,
        beta: Value,
        in_check: bool,
        is_pv: bool,
    ) -> bool {
        !is_pv
            && !in_check
            && depth >= Self::NULL_MIN_DEPTH
            && board.has_non_pawn_material()
            && board.eval() >= beta
    }

    #[inline(always)]
    fn can_apply_rfp(depth: Depth, in_check: bool, is_pv: bool, beta: Value) -> bool {
        depth <= Self::RFP_MAX_DEPTH && !in_check && !is_pv && !Self::is_checkmate(beta)
    }

    #[inline(always)]
    fn can_apply_lmr(m: Move, depth: Depth, move_index: usize) -> bool {
        depth >= Self::LMR_MIN_DEPTH && move_index >= Self::LMR_MOVE_WO_REDUCTION && m.is_quiet()
    }

    #[inline(always)]
    fn null_reduction(depth: Depth) -> Depth {
        // Idea of dividing in null move depth taken from Cosette
        Self::NULL_MIN_DEPTH_REDUCTION + (depth - Self::NULL_MIN_DEPTH) / Self::NULL_DEPTH_DIVIDER
    }

    #[inline(always)]
    fn rfp_margin(depth: Depth) -> Value {
        Self::RFP_MARGIN_MULTIPLIER * (depth as Value)
    }

    #[inline(always)]
    fn late_move_reduction(depth: Depth, move_index: usize) -> Depth {
        // LMR table idea from Ethereal
        unsafe { LMR_TABLE[min(depth as usize, 63)][min(move_index, 63)] }
    }

    fn is_checkmate(value: Value) -> bool {
        value.abs() >= Value::MAX >> 1
    }

    fn get_pv(&self, board: &mut Board, depth: Depth) -> String {
        if depth == 0 {
            return String::new();
        }

        if let Some(tt_entry) = self.tt.probe(board) {
            if let Some(hash_move) = tt_entry.best_move() {
                let mut pv = String::new();
                if MoveList::from(board).contains(hash_move) {
                    board.push(hash_move);
                    pv = format!("{} {}", hash_move, self.get_pv(board, depth - 1));
                    board.pop();
                }
                return pv;
            }
        }
        String::new()
    }

    fn print_info(&self, board: &mut Board, depth: Depth, m: Option<Move>, score: Value) {
        if m.is_none() || self.id != 0 || self.stop {
            return;
        }

        let score_str = if Self::is_checkmate(score) {
            let mate_score = if score > 0 {
                (Value::MAX - score + 1) / 2
            } else {
                -(score + Value::MAX) / 2
            };
            format!("mate {}", mate_score)
        } else {
            format!("cp {}", score)
        };

        println!("info currmove {m} depth {depth} seldepth {sel_depth} time {time} score {score_str} nodes {nodes} nps {nps} hashfull {hashfull} pv {pv}",
                 m = m.unwrap(),
                 depth = depth,
                 sel_depth = self.sel_depth,
                 time = self.timer.elapsed(),
                 score_str = score_str,
                 nodes = self.nodes,
                 nps = 1000 * self.nodes / (self.timer.elapsed() + 1),
                 hashfull = self.tt.hashfull(),
                 pv = self.get_pv(board, depth));
    }

    fn print_currmovenumber(depth: Depth, m: Move, idx: usize) {
        println!(
            "info depth {depth} currmove {currmove} currmovenumber {currmovenumber}",
            depth = depth,
            currmove = m,
            currmovenumber = idx + 1,
        )
    }
}

impl<'a> Search<'a> {
    const PRINT_CURRMOVENUMBER_TIME_MILLIS: Time = 3000;
    const SEARCHES_WO_TIMER_UPDATE: Depth = 4;
    const RFP_MAX_DEPTH: Depth = 8;
    const RFP_MARGIN_MULTIPLIER: Value = 120;
    const ASPIRATION_WINDOW: Value = 25;
    const NULL_MIN_DEPTH: Depth = 2;
    const NULL_MIN_DEPTH_REDUCTION: Depth = 3;
    const NULL_DEPTH_DIVIDER: Depth = 4;
    const LMR_MOVE_WO_REDUCTION: usize = 2;
    const LMR_MIN_DEPTH: Depth = 2;
    const LMR_BASE_REDUCTION: f32 = 0.75;
    const LMR_MOVE_DIVIDER: f32 = 2.25;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

pub static mut LMR_TABLE: [[Depth; 64]; 64] = [[0; 64]; 64];

fn init_lmr_table(lmr_table: &mut [[Depth; 64]; 64]) {
    for depth in 1..64 {
        for move_number in 1..64 {
            lmr_table[depth][move_number] = (Search::LMR_BASE_REDUCTION
                + f32::ln(depth as f32) * f32::ln(move_number as f32) / Search::LMR_MOVE_DIVIDER)
                as Depth;
        }
    }
}

pub fn init_search() {
    unsafe {
        init_lmr_table(&mut LMR_TABLE);
    }
}