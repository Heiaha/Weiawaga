use super::move_sorter::*;
use super::statistics::*;
use super::timer::*;
use super::tt::*;
use crate::evaluation::eval::*;
use crate::evaluation::score::*;
use crate::types::board::*;
use crate::types::moov::*;
use std::cmp::{max, min};

pub type Depth = i8;
pub type Ply = usize;

pub struct Search<'a> {
    stop: bool,
    sel_depth: Ply,
    timer: Timer,
    tt: &'a mut TT,
    stats: Statistics,
}

impl<'a> Search<'a> {
    pub fn new(timer: Timer, tt: &'a mut TT) -> Self {
        Search { stop: false,
                 sel_depth: 0,
                 timer: timer,
                 tt: tt,
                 stats: Statistics::new() }
    }

    pub fn go(&mut self, board: &mut Board) -> (Move, Value) {

        ///////////////////////////////////////////////////////////////////
        // Starts iterative deepening.
        ///////////////////////////////////////////////////////////////////
        let mut alpha = -Score::INF;
        let mut beta = Score::INF;
        let mut depth = 1;
        let mut final_move = Move::NULL;
        let mut final_score = 0;
        let mut last_score = 0;

        let move_sorter = MoveSorter::new(board, 0, &None);

        ///////////////////////////////////////////////////////////////////
        // If there's only one legal move, just play
        // it instead of searching.
        ///////////////////////////////////////////////////////////////////
        if move_sorter.len() == 1 {
            return (move_sorter[0], 0);
        }

        while !self.stop && self.timer.start_check(depth) && !Score::is_checkmate(final_score) && depth < Depth::MAX {
            (final_move, final_score) = self.search_root(board, depth, alpha, beta);


            ///////////////////////////////////////////////////////////////////
            // Update the clock if the score is changing
            // by a lot.
            ///////////////////////////////////////////////////////////////////
            if depth >= 4 {
                self.timer.update(final_score - last_score);
            }
            last_score = final_score;


            ///////////////////////////////////////////////////////////////////
            // Widen aspiration windows.
            ///////////////////////////////////////////////////////////////////
            if final_score <= alpha {
                alpha = -Score::INF;
            } else if final_score >= beta {
                beta = Score::INF;
            } else {
                self.print_info(board, depth, final_move, final_score);
                alpha = final_score - Self::ASPIRATION_WINDOW;
                beta = final_score + Self::ASPIRATION_WINDOW;
                depth += 1;
                self.stats = Statistics::new();
            }
        }
        MoveSorter::clear_history();
        MoveSorter::clear_killers();
        (final_move, final_score)
    }

    pub fn search_root(&mut self, board: &mut Board, mut depth: Depth, mut alpha: Value, beta: Value) -> (Move, Value) {
        let ply: Ply = 0;

        ///////////////////////////////////////////////////////////////////
        // Check extension. Since we've already generated
        // legal moves, we can just use the fact that
        // this is stored.
        ///////////////////////////////////////////////////////////////////
        if board.in_check() {
            depth += 1;
        }

        ///////////////////////////////////////////////////////////////////
        // Check the hash table for the current
        // position, primarily for move ordering.
        ///////////////////////////////////////////////////////////////////
        let mut hash_move = None;
        if let Some(tt_entry) = self.tt.probe(board.hash()) {
            hash_move = tt_entry.best_move();
        }

        ///////////////////////////////////////////////////////////////////
        // Score moves and begin searching recursively.
        ///////////////////////////////////////////////////////////////////
        let mut value: Value = -Score::INF;
        let mut best_move: Move = Move::NULL;
        let mut idx = 0;
        let mut move_sorter = MoveSorter::new(board, ply, &hash_move);
        while let Some(m) = move_sorter.next() {

            board.push(m);
            if idx == 0 || -self.search(board, depth - 1, ply + 1, -alpha - 1, -alpha, true, false) > alpha {
                value = -self.search(board, depth - 1, ply + 1, -beta, -alpha, true, true);
            }
            board.pop();

            if self.stop || self.timer.stop_check() {
                self.stop = true;
                break;
            }

            if value > alpha {
                best_move = m;
                if value >= beta {
                    self.tt.insert(board.hash(), depth, beta, Some(best_move), TTFlag::Lower);
                    return (best_move, beta);
                }
                alpha = value;
                self.tt.insert(board.hash(), depth, alpha, Some(best_move), TTFlag::Upper);
            }
            idx += 1;
        }

        if best_move == Move::NULL {
            best_move = move_sorter[0];
        }

        if !self.stop {
            self.tt.insert(board.hash(), depth, alpha, Some(best_move), TTFlag::Exact);
        }
        (best_move, alpha)
    }

    fn search(&mut self, board: &mut Board, mut depth: Depth, ply: Ply, mut alpha: Value, mut beta: Value, can_apply_null: bool, is_pv: bool) -> Value {
        if self.stop || self.timer.stop_check() {
            self.stop = true;
            return 0;
        }

        ///////////////////////////////////////////////////////////////////
        // Mate distance pruning - will help reduce
        // some nodes when checkmate is near.
        ///////////////////////////////////////////////////////////////////
        let mate_value = Score::INF - (ply as Value);
        alpha = max(alpha, -mate_value);
        beta = min(beta, mate_value - 1);
        if alpha >= beta {
            self.stats.leafs += 1;
            return alpha;
        }

        ///////////////////////////////////////////////////////////////////
        // Extend search if position is in check.
        ///////////////////////////////////////////////////////////////////
        let in_check = board.in_check();
        if in_check {
            depth += 1;
        }
        depth = max(0, depth);

        ///////////////////////////////////////////////////////////////////
        // Quiescence search - here we search tactical
        // moves after the main search to prevent a
        // horizon effect.
        ///////////////////////////////////////////////////////////////////
        if depth == 0 {
            return self.q_search(board, ply, alpha, beta);
        }
        self.stats.nodes += 1;

        if board.is_draw() {
            self.stats.leafs += 1;
            return 0;
        }

        ///////////////////////////////////////////////////////////////////
        // Probe the hash table and adjust the value.
        // If appropriate, produce a cutoff.
        ///////////////////////////////////////////////////////////////////
        let mut hash_move: Option<Move> = None;
        if let Some(tt_entry) = self.tt.probe(board.hash()) {
            if tt_entry.depth() >= depth {
                self.stats.tt_hits += 1;
                match tt_entry.flag() {
                    TTFlag::Exact => {
                        self.stats.leafs += 1;
                        return tt_entry.value();
                    }
                    TTFlag::Lower => {
                        alpha = max(alpha, tt_entry.value());
                    }
                    TTFlag::Upper => {
                        beta = min(beta, tt_entry.value());
                    }
                }
                if alpha >= beta {
                    self.stats.leafs += 1;
                    self.stats.beta_cutoffs += 1;
                    return tt_entry.value();
                }
            }
            hash_move = tt_entry.best_move();
        }

        ///////////////////////////////////////////////////////////////////
        // Null move pruning.
        ///////////////////////////////////////////////////////////////////
        if Self::can_apply_null(board, depth, beta, in_check, can_apply_null) {
            let r = Self::null_reduction(depth);
            board.push_null();
            let value = -self.search(board, depth - r - 1, ply, -beta, -beta + 1, false, false);
            board.pop_null();
            if self.stop {
                return 0;
            }
            if value >= beta {
                self.stats.beta_cutoffs += 1;
                return beta;
            }
        }

        ///////////////////////////////////////////////////////////////////
        // Generate moves, score, and begin searching
        // recursively.
        ///////////////////////////////////////////////////////////////////
        let mut value: Value;
        let mut reduced_depth: Depth;
        let mut tt_flag = TTFlag::Upper;
        let mut best_move: Option<Move> = None;
        let mut idx = 0;

        let mut move_sorter = MoveSorter::new(board, ply, &hash_move);
        while let Some(m) = move_sorter.next() {

            ///////////////////////////////////////////////////////////////////
            // Make move and deepen search via principal variation search.
            ///////////////////////////////////////////////////////////////////
            board.push(m);

            if idx == 0 {
                value = -self.search(board, depth - 1, ply + 1, -beta, -alpha, true, is_pv);
            } else {

                ///////////////////////////////////////////////////////////////////
                // Late move reductions.
                ///////////////////////////////////////////////////////////////////
                reduced_depth = depth;
                if Self::can_apply_lmr(&m, depth, idx) {
                    reduced_depth -= Self::late_move_reduction(depth, idx);
                }
                loop {
                    value = -self.search(board, reduced_depth - 1, ply + 1, -alpha - 1, -alpha, true, false);
                    if value > alpha {
                        value = -self.search(board, reduced_depth - 1, ply + 1, -beta, -alpha, true, true);
                    }

                    ///////////////////////////////////////////////////////////////////
                    // A reduced depth may bring us above alpha. This is relatively
                    // unusual, but if so we need the exact score so we do a full search.
                    ///////////////////////////////////////////////////////////////////
                    if reduced_depth != depth && value > alpha {
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
                    if m.flags() == MoveFlags::Quiet {
                        MoveSorter::add_killer(board, m, ply);
                        MoveSorter::add_history(m, depth);
                    }
                    self.stats.beta_cutoffs += 1;
                    tt_flag = TTFlag::Lower;
                    alpha = beta;
                    break;
                }
                tt_flag = TTFlag::Exact;
                alpha = value;
            }
            idx += 1;
        }

        ///////////////////////////////////////////////////////////////////
        // Checkmate and stalemate check.
        ///////////////////////////////////////////////////////////////////
        if move_sorter.len() == 0 {
            if in_check {
                alpha = -mate_value;
            } else {
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

        let mut value = eval(board);

        if value >= beta {
            self.stats.qleafs += 1;
            return beta;
        }
        alpha = max(alpha, value);

        let mut hash_move: Option<Move> = None;
        if let Some(tt_entry) = self.tt.probe(board.hash()) {
            hash_move = tt_entry.best_move();
        }

        let mut move_sorter = MoveSorter::new_q(board, ply, &hash_move);
        while let Some(m) = move_sorter.next() {
            board.push(m);
            value = -self.q_search(board,
                                   ply + 1,
                                   -beta,
                                   -alpha);
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
        can_apply_null && !in_check && depth >= Self::NULL_MIN_DEPTH && board.has_non_pawn_material() && eval(board) >= beta
    }

    #[inline(always)]
    fn can_apply_lmr(m: &Move, depth: Depth, move_index: usize) -> bool {
        depth >= Self::LMR_MIN_DEPTH && move_index >= Self::LMR_MOVE_WO_REDUCTION && m.flags() == MoveFlags::Quiet
    }

    #[inline(always)]
    fn null_reduction(depth: Depth) -> Depth {
        // Idea of dividing in null move depth taken from Cosette
        Self::NULL_MIN_DEPTH_REDUCTION + (depth - Self::NULL_MIN_DEPTH)/Self::NULL_DEPTH_DIVIDER
    }

    #[inline(always)]
    fn late_move_reduction(depth: Depth, move_index: usize) -> Depth {
        // LMR table idea from Ethereal
        unsafe { LMR_TABLE[min(depth as usize, 63)][min(move_index, 63)] }
    }

    fn get_pv(&self, board: &mut Board, depth: Depth) -> String {
        if depth == 0 {
            return "".to_owned();
        }
        let hash_move: Option<Move>;
        let tt_entry = self.tt.probe(board.hash());
        match tt_entry {
            Some(tt_entry) => {
                hash_move = tt_entry.best_move();
                if hash_move == None {
                    return "".to_owned();
                }
            }
            None => {
                return "".to_owned();
            }
        }

        board.push(hash_move.unwrap());
        let pv = hash_move.unwrap().to_string() + " " + &*self.get_pv(board, depth - 1);
        board.pop();
        pv
    }

    fn print_info(&self, board: &mut Board, depth: Depth, m: Move, score: Value) {
        println!("info currmove {m} depth {depth} seldepth {sel_depth} time {time} score cp {score} nodes {nodes} nps {nps} pv {pv}",
                 m = m.to_string(),
                 depth = depth,
                 sel_depth = self.sel_depth,
                 time = self.timer.elapsed(),
                 score = score,
                 nodes = self.stats.total_nodes(),
                 nps = 1000 * self.stats.total_nodes() / (self.timer.elapsed() + 1),
                 pv = self.get_pv(board, depth));
    }
}

impl<'a> Search<'a> {
    const ASPIRATION_WINDOW: Value = 25;

    const NULL_MIN_DEPTH: Depth = 2;
    const NULL_MIN_DEPTH_REDUCTION: Depth = 3;
    const NULL_DEPTH_DIVIDER: Depth = 4;

    const LMR_MOVE_WO_REDUCTION: usize = 2;
    const LMR_MIN_DEPTH: Depth = 3;
    const LMR_BASE_REDUCTION: f32 = 0.75;
    const LMR_MOVE_DIVIDER: f32 = 2.25;
}

pub static mut LMR_TABLE: [[Depth; 64]; 64] = [[0; 64]; 64];

fn init_lmr_table(lmr_table: &mut [[Depth; 64]; 64]) {
    for depth in 1..64 {
        for move_number in 1..64 {
            lmr_table[depth][move_number] = (Search::LMR_BASE_REDUCTION + f32::ln(depth as f32) * f32::ln(move_number as f32) / Search::LMR_MOVE_DIVIDER) as Depth;
        }
    }
}

pub fn init_search() {
    unsafe {
        init_lmr_table(&mut LMR_TABLE);
    }
}
