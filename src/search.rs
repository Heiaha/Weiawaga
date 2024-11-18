use std::time::Duration;

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

    pub fn go(&mut self, mut board: Board) -> Option<Move> {
        ///////////////////////////////////////////////////////////////////
        // Start iterative deepening.
        ///////////////////////////////////////////////////////////////////
        let mut alpha = -Self::MATE;
        let mut beta = Self::MATE;
        let mut best_move = None;
        let mut value = 0;
        let mut depth = 1;

        ///////////////////////////////////////////////////////////////////
        // If there's only one legal move, just play
        // it instead of searching.
        ///////////////////////////////////////////////////////////////////
        let moves = MoveList::from(&board);

        if moves.len() == 0 {
            return None;
        }

        if moves.len() == 1 {
            return Some(moves[0].m);
        }

        while !self.stop
            && self.timer.start_check(depth)
            && !Self::is_checkmate(value)
            && depth < Depth::MAX
        {
            (best_move, value) = self.search_root(&mut board, depth, alpha, beta);

            ///////////////////////////////////////////////////////////////////
            // Update the clock if the score is changing
            // by a lot.
            ///////////////////////////////////////////////////////////////////
            if depth >= Self::SEARCHES_WO_TIMER_UPDATE {
                self.timer.update(best_move);
            }

            ///////////////////////////////////////////////////////////////////
            // Widen aspiration windows.
            ///////////////////////////////////////////////////////////////////
            if value <= alpha {
                alpha = -Self::MATE;
            } else if value >= beta {
                beta = Self::MATE;
            } else {
                // Only print info if we're in the main thread
                if self.id == 0 && !self.stop {
                    best_move.inspect(|&m| self.print_info(&mut board, depth, m, value));
                }
                alpha = value - Self::ASPIRATION_WINDOW;
                beta = value + Self::ASPIRATION_WINDOW;
                depth += 1;
                self.sel_depth = 0;
            }
        }

        if self.id == 0 {
            self.timer.stop();
        }

        best_move
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
        let hash_move = self.tt.probe(board).and_then(|entry| entry.best_move());

        ///////////////////////////////////////////////////////////////////
        // Score moves and begin searching recursively.
        ///////////////////////////////////////////////////////////////////
        let ply = 0;
        let mut value = -Self::MATE;
        let mut best_move = None;
        let mut idx = 0;

        let mut moves = MoveList::from(board);
        self.move_sorter
            .score_moves(&mut moves, board, ply, hash_move);

        while let Some(m) = moves.next_best(idx) {
            if self.id == 0 && self.timer.elapsed() >= Self::PRINT_CURRMOVENUMBER_TIME {
                Self::print_currmovenumber(depth, m, idx);
            }

            board.push(m);
            if idx == 0 || -self.search(board, depth - 1, -alpha - 1, -alpha, ply + 1) > alpha {
                value = -self.search(board, depth - 1, -beta, -alpha, ply + 1);
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

        best_move = best_move.or_else(|| {
            if moves.len() > 0 {
                Some(moves[0].m)
            } else {
                None
            }
        });

        if !self.stop {
            self.tt.insert(board, depth, alpha, best_move, Bound::Exact);
        }
        (best_move, alpha)
    }

    fn search(
        &mut self,
        board: &mut Board,
        mut depth: Depth,
        mut alpha: Value,
        mut beta: Value,
        ply: Ply,
    ) -> Value {
        if self.stop || self.timer.stop_check() {
            self.stop = true;
            return 0;
        }

        ///////////////////////////////////////////////////////////////////
        // Mate distance pruning - will help reduce
        // some nodes when checkmate is near.
        ///////////////////////////////////////////////////////////////////
        let mate_value = Self::MATE - (ply as Value);
        alpha = alpha.max(-mate_value);
        beta = beta.min(mate_value - 1);
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
            return self.q_search(board, alpha, beta, ply);
        }

        self.nodes += 1;

        if board.is_draw() {
            return 0;
        }

        ///////////////////////////////////////////////////////////////////
        // Check if we're in a pv node
        ///////////////////////////////////////////////////////////////////
        let is_pv = alpha != beta - 1;

        ///////////////////////////////////////////////////////////////////
        // Probe the hash table and adjust the value.
        // If appropriate, produce a cutoff.
        ///////////////////////////////////////////////////////////////////
        let tt_entry = self.tt.probe(board);
        if let Some(tt_entry) = tt_entry {
            if tt_entry.depth() >= depth && !is_pv {
                match tt_entry.flag() {
                    Bound::Exact => return tt_entry.value(),
                    Bound::Lower => alpha = alpha.max(tt_entry.value()),
                    Bound::Upper => beta = beta.min(tt_entry.value()),
                }
                if alpha >= beta {
                    return tt_entry.value();
                }
            }
        } else if Self::can_apply_iid(depth, in_check, is_pv) {
            depth -= Self::IID_DEPTH_REDUCTION;
        }

        ///////////////////////////////////////////////////////////////////
        // Reverse Futility Pruning
        ///////////////////////////////////////////////////////////////////
        if Self::can_apply_rfp(depth, in_check, is_pv, beta) {
            let eval = tt_entry.map_or_else(|| board.eval(), |entry| entry.value());

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
            let value = -self.search(board, depth - r - 1, -beta, -beta + 1, ply);
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
        let mut tt_flag = Bound::Upper;
        let mut best_move = None;
        let mut idx = 0;

        let mut moves = MoveList::from(board);
        self.move_sorter.score_moves(
            &mut moves,
            board,
            ply,
            tt_entry.and_then(|entry| entry.best_move()),
        );

        while let Some(m) = moves.next_best(idx) {
            ///////////////////////////////////////////////////////////////////
            // Make move and deepen search via principal variation search.
            ///////////////////////////////////////////////////////////////////
            board.push(m);

            if depth > 1 {
                self.tt.prefetch(board);
            }

            let mut value;
            if idx == 0 {
                value = -self.search(board, depth - 1, -beta, -alpha, ply + 1);
            } else {
                ///////////////////////////////////////////////////////////////////
                // Late move reductions.
                ///////////////////////////////////////////////////////////////////
                let mut reduction = if Self::can_apply_lmr(m, depth, idx) {
                    Self::late_move_reduction(depth, idx)
                } else {
                    0
                };

                loop {
                    value = -self.search(board, depth - reduction - 1, -alpha - 1, -alpha, ply + 1);
                    if value > alpha {
                        value = -self.search(board, depth - reduction - 1, -beta, -alpha, ply + 1);
                    }

                    ///////////////////////////////////////////////////////////////////
                    // A reduced depth may bring us above alpha. This is relatively
                    // unusual, but if so we need the exact score so we do a full search.
                    ///////////////////////////////////////////////////////////////////
                    if reduction > 0 && value > alpha {
                        reduction = 0;
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
            best_move = best_move.or_else(|| {
                if moves.len() > 0 {
                    Some(moves[0].m)
                } else {
                    None
                }
            });

            self.tt.insert(board, depth, alpha, best_move, tt_flag);
        }
        alpha
    }

    fn q_search(
        &mut self,
        board: &mut Board,
        mut alpha: Value,
        mut beta: Value,
        ply: Ply,
    ) -> Value {
        if self.stop || self.timer.stop_check() {
            self.stop = true;
            return 0;
        }

        self.nodes += 1;

        if board.is_draw() {
            return 0;
        }

        self.sel_depth = self.sel_depth.max(ply);

        let eval = board.eval();

        if eval >= beta {
            return beta;
        }
        alpha = alpha.max(eval);

        let tt_entry = self.tt.probe(board);
        if let Some(tt_entry) = tt_entry {
            match tt_entry.flag() {
                Bound::Exact => return tt_entry.value(),
                Bound::Lower => alpha = alpha.max(tt_entry.value()),
                Bound::Upper => beta = beta.min(tt_entry.value()),
            }
            if alpha >= beta {
                return tt_entry.value();
            }
        }

        let mut moves = MoveList::from_q(board);
        self.move_sorter.score_moves(
            &mut moves,
            board,
            ply,
            tt_entry.and_then(|entry| entry.best_move()),
        );

        let mut idx = 0;
        while let Some(m) = moves.next_best(idx) {
            ///////////////////////////////////////////////////////////////////
            // Effectively a SEE check. Bad captures will have a score < 0
            // given by the SEE + the bad capture offset,
            // and here we skip bad captures.
            ///////////////////////////////////////////////////////////////////
            if moves[idx].score < 0 {
                break;
            }

            board.push(m);
            let value = -self.q_search(board, -beta, -alpha, ply + 1);
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

    fn can_apply_null(
        board: &Board,
        depth: Depth,
        beta: Value,
        in_check: bool,
        is_pv: bool,
    ) -> bool {
        !is_pv
            && !in_check
            && board.peek().is_some()
            && depth >= Self::NULL_MIN_DEPTH
            && board.has_non_pawn_material()
            && board.eval() >= beta
            && !Self::is_checkmate(beta)
    }

    fn can_apply_iid(depth: Depth, in_check: bool, is_pv: bool) -> bool {
        depth >= Self::IID_MIN_DEPTH && !in_check && !is_pv
    }

    fn can_apply_rfp(depth: Depth, in_check: bool, is_pv: bool, beta: Value) -> bool {
        depth <= Self::RFP_MAX_DEPTH && !in_check && !is_pv && !Self::is_checkmate(beta)
    }

    fn can_apply_lmr(m: Move, depth: Depth, move_index: usize) -> bool {
        depth >= Self::LMR_MIN_DEPTH && move_index >= Self::LMR_MOVE_WO_REDUCTION && m.is_quiet()
    }

    fn null_reduction(depth: Depth) -> Depth {
        // Idea of dividing in null move depth taken from Cosette
        Self::NULL_MIN_DEPTH_REDUCTION + (depth - Self::NULL_MIN_DEPTH) / Self::NULL_DEPTH_DIVIDER
    }

    fn rfp_margin(depth: Depth) -> Value {
        Self::RFP_MARGIN_MULTIPLIER * (depth as Value)
    }

    fn late_move_reduction(depth: Depth, move_index: usize) -> Depth {
        // LMR table idea from Ethereal
        unsafe { LMR_TABLE[depth.min(63) as usize][move_index.min(63)] }
    }

    fn is_checkmate(value: Value) -> bool {
        value.abs() >= Self::MATE >> 1
    }

    fn get_pv(&self, board: &mut Board, depth: Depth) -> String {
        if depth == 0 {
            return String::new();
        }

        if let Some(tt_entry) = self.tt.probe(board) {
            let mut pv = String::new();
            if let Some(m) = tt_entry
                .best_move()
                .filter(|&m| MoveList::from(board).contains(m))
            {
                board.push(m);
                pv = format!("{} {}", m, self.get_pv(board, depth - 1));
                board.pop();
            }
            return pv;
        }
        String::new()
    }

    fn print_info(&self, board: &mut Board, depth: Depth, m: Move, value: Value) {
        let score_str = if Self::is_checkmate(value) {
            let mate_value = if value > 0 {
                (Self::MATE - value + 1) / 2
            } else {
                -(value + Self::MATE) / 2
            };
            format!("mate {}", mate_value)
        } else {
            format!("cp {}", value)
        };

        let elapsed = self.timer.elapsed();

        println!("info currmove {m} depth {depth} seldepth {sel_depth} time {time} score {score_str} nodes {nodes} nps {nps} pv {pv}",
                 m = m,
                 depth = depth,
                 sel_depth = self.sel_depth,
                 time = elapsed.as_millis(),
                 score_str = score_str,
                 nodes = self.nodes,
                 nps = (self.nodes as f64 / elapsed.as_secs_f64()) as u64,
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
    const PRINT_CURRMOVENUMBER_TIME: Duration = Duration::from_millis(3000);
    const SEARCHES_WO_TIMER_UPDATE: Depth = 8;
    const RFP_MAX_DEPTH: Depth = 9;
    const RFP_MARGIN_MULTIPLIER: Value = 63;
    const ASPIRATION_WINDOW: Value = 61;
    const NULL_MIN_DEPTH: Depth = 2;
    const NULL_MIN_DEPTH_REDUCTION: Depth = 1;
    const NULL_DEPTH_DIVIDER: Depth = 2;
    const IID_MIN_DEPTH: Depth = 7;
    const IID_DEPTH_REDUCTION: Depth = 2;
    const LMR_MOVE_WO_REDUCTION: usize = 3;
    const LMR_MIN_DEPTH: Depth = 2;
    const LMR_BASE_REDUCTION: f32 = 0.11;
    const LMR_MOVE_DIVIDER: f32 = 1.56;
    const MATE: Value = 32000;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

pub static mut LMR_TABLE: [[Depth; 64]; 64] = [[0; 64]; 64];

fn init_lmr_table() -> [[Depth; 64]; 64] {
    let mut lmr_table = [[0; 64]; 64];
    for depth in 1..64 {
        for move_number in 1..64 {
            lmr_table[depth][move_number] = (Search::LMR_BASE_REDUCTION
                + f32::ln(depth as f32) * f32::ln(move_number as f32) / Search::LMR_MOVE_DIVIDER)
                as Depth;
        }
    }
    lmr_table
}

pub fn init_search() {
    unsafe {
        LMR_TABLE = init_lmr_table();
    }
}
