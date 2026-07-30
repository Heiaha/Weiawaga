use std::sync::LazyLock;
use std::time::Duration;

use arrayvec::ArrayVec;

use super::board::*;
use super::moov::*;
use super::move_list::*;
use super::move_sorting::*;
use super::timer::*;
use super::tt::*;
use super::types::*;

pub struct RootMove {
    pub m: Move,
    pub value: i32,
    pub pv: Vec<Move>,
    pub sel_depth: usize,
}

impl RootMove {
    fn new(m: Move) -> Self {
        Self {
            m,
            value: -i32::MATE,
            pv: Vec::new(),
            sel_depth: 0,
        }
    }
}

pub struct Search<'a> {
    id: u16,
    sel_depth: usize,
    show_wdl: bool,
    multi_pv: usize,
    timer: Timer,
    tt: &'a TT,
    scorer: MoveScorer,
    excluded_moves: [Option<Move>; MAX_PLY],
    eval_stack: [i32; MAX_PLY],
    pv_table: Vec<Vec<Move>>,
}

impl<'a> Search<'a> {
    pub fn new(timer: Timer, tt: &'a TT, id: u16, show_wdl: bool, multi_pv: usize) -> Self {
        Self {
            id,
            timer,
            tt,
            show_wdl,
            multi_pv,
            sel_depth: 0,
            scorer: MoveScorer::new(),
            excluded_moves: [None; MAX_PLY],
            eval_stack: [0; MAX_PLY],
            pv_table: vec![Vec::new(); MAX_PLY],
        }
    }

    pub fn go(&mut self, mut board: Board) -> (Option<Move>, Option<Move>) {
        let mut moves = MoveList::from::<false>(&board);
        if moves.is_empty() {
            return (None, None);
        }

        ///////////////////////////////////////////////////////////////////
        // Build the root move list in move-ordering order. The list carries
        // each move's standing across depths: the first multi_pv entries are
        // the reported lines, and each line searches the tail slice from its
        // own slot down.
        ///////////////////////////////////////////////////////////////////
        let hash_move = self.tt.get(&board, 0).and_then(|entry| entry.best_move());
        let mut root_moves = self
            .scorer
            .create_sorter::<false>(&mut moves, &board, 0, hash_move)
            .map(RootMove::new)
            .collect::<Vec<_>>();

        let multi_pv = self.multi_pv.min(root_moves.len());

        'deepening: for depth in 1..i8::MAX {
            if !self.timer.start_check(Some(root_moves[0].m), depth) {
                break;
            }

            for pv_idx in 0..multi_pv {
                let (value, bound) = self.aspiration(&mut board, depth, &mut root_moves[pv_idx..]);
                if self.timer.is_stopped() {
                    break 'deepening;
                }
                // A tail search covers only part of the root moves; only the
                // full-width line may write the root position's entry.
                if pv_idx == 0 {
                    self.tt
                        .insert(&board, depth, value, Some(root_moves[0].m), bound, 0);
                }
                self.sel_depth = 0;
            }

            root_moves[..multi_pv].sort_by_key(|line| std::cmp::Reverse(line.value));

            if self.id == 0 && !self.timer.is_stopped() {
                for (pv_idx, line) in root_moves[..multi_pv].iter().enumerate() {
                    self.print_info(
                        &mut board,
                        depth,
                        line,
                        (multi_pv > 1).then_some(pv_idx + 1),
                    );
                }
            }
        }

        let best_move = root_moves[0].m;

        if self.id == 0 {
            self.timer.set_stop();
        }

        // Ensure the ponder move from the last pv is still legal.
        // It could be illegal if the last search was only partially completed and the best_move had changed.
        let ponder_move = root_moves[0].pv.get(1).copied().and_then(|ponder_move| {
            board.push(best_move);
            let m = MoveList::from::<false>(&board)
                .contains(ponder_move)
                .then_some(ponder_move);
            board.pop();
            m
        });

        (Some(best_move), ponder_move)
    }

    fn aspiration(&mut self, board: &mut Board, depth: i8, lines: &mut [RootMove]) -> (i32, Bound) {
        if depth < Self::ASPIRATION_MIN_DEPTH {
            return self.search_root(board, depth, -i32::MATE, i32::MATE, lines);
        }

        let pred = lines[0].value;
        let alpha = (pred - Self::ASPIRATION_WINDOW).max(-i32::MATE);
        let beta = (pred + Self::ASPIRATION_WINDOW).min(i32::MATE);

        let (value, bound) = self.search_root(board, depth, alpha, beta, lines);

        if value <= alpha {
            self.search_root(board, depth, -i32::MATE, beta, lines)
        } else if value >= beta {
            self.search_root(board, depth, alpha, i32::MATE, lines)
        } else {
            (value, bound)
        }
    }

    fn search_root(
        &mut self,
        board: &mut Board,
        mut depth: i8,
        mut alpha: i32,
        beta: i32,
        lines: &mut [RootMove],
    ) -> (i32, Bound) {
        ///////////////////////////////////////////////////////////////////
        // Clear the pv line and excluded moves.
        ///////////////////////////////////////////////////////////////////
        self.pv_table.iter_mut().for_each(|line| line.clear());
        self.excluded_moves.fill(None);

        ///////////////////////////////////////////////////////////////////
        // Check extension.
        ///////////////////////////////////////////////////////////////////
        if board.in_check() {
            depth += 1;
        }

        // Seed the eval stack so nodes at ply 2 have a valid improving check.
        self.eval_stack[0] = if board.in_check() {
            -i32::MATE
        } else {
            board.eval()
        };

        let mut best_idx = 0;
        let mut best_value = -i32::MATE;
        let mut tt_flag = Bound::Upper;
        let mut value = 0;

        for (idx, line) in lines.iter().enumerate() {
            let m = line.m;

            if self.id == 0
                && self.timer.elapsed() >= Self::PRINT_CURRMOVENUMBER_TIME
                && !self.timer.is_stopped()
            {
                Self::print_currmovenumber(depth, m, idx);
            }

            board.push(m);
            if idx == 0 || -self.search(board, depth - 1, -alpha - 1, -alpha, 1) > alpha {
                value = -self.search(board, depth - 1, -beta, -alpha, 1)
            };
            board.pop();

            if self.timer.is_stopped() {
                break;
            }

            self.timer.update_node_table(m);

            if value > best_value {
                best_value = value;
                best_idx = idx;

                if value > alpha {
                    self.update_pv(m, 0);
                    if value >= beta {
                        tt_flag = Bound::Lower;
                        break;
                    }
                    alpha = value;
                    tt_flag = Bound::Exact;
                }
            }
        }

        ///////////////////////////////////////////////////////////////////
        // Promote the winner to the front of the slice (the rest keep
        // their order) with its rebuilt standing. A stopped or fail-low
        // search produces no pv, and the winner keeps its old line then.
        ///////////////////////////////////////////////////////////////////
        let winner = RootMove {
            m: lines[best_idx].m,
            value: best_value,
            sel_depth: self.sel_depth,
            pv: if self.pv_table[0].is_empty() {
                lines[best_idx].pv.clone()
            } else {
                self.pv_table[0].clone()
            },
        };
        lines[..=best_idx].rotate_right(1);
        lines[0] = winner;

        (best_value, tt_flag)
    }

    fn search(
        &mut self,
        board: &mut Board,
        mut depth: i8,
        mut alpha: i32,
        mut beta: i32,
        ply: usize,
    ) -> i32 {
        ///////////////////////////////////////////////////////////////////
        // Clear the pv line.
        ///////////////////////////////////////////////////////////////////
        self.pv_table[ply].clear();
        self.sel_depth = self.sel_depth.max(ply);

        ///////////////////////////////////////////////////////////////////
        // Mate distance pruning - will help reduce
        // some nodes when checkmate is near.
        ///////////////////////////////////////////////////////////////////
        let mate_value = i32::MATE - (ply as i32);
        alpha = alpha.max(-mate_value);
        beta = beta.min(mate_value - 1);
        if alpha >= beta {
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

        if self.timer.stop_check() {
            return 0;
        }

        if board.is_draw() {
            return 0;
        }

        ///////////////////////////////////////////////////////////////////
        // Check if we're in a pv node
        ///////////////////////////////////////////////////////////////////
        let is_pv = alpha != beta - 1;
        let excluded_move = self.excluded_moves[ply];

        ///////////////////////////////////////////////////////////////////
        // Probe the hash table and adjust the value.
        // If appropriate, produce a cutoff.
        ///////////////////////////////////////////////////////////////////
        let tt_entry = self.tt.get(board, ply);
        if let Some(tt_entry) = tt_entry
            && tt_entry.depth() >= depth
            && !is_pv
            && excluded_move.is_none()
        {
            let tt_value = tt_entry.value();

            match tt_entry.bound() {
                Bound::Exact => return tt_value,
                Bound::Lower => alpha = alpha.max(tt_value),
                Bound::Upper => beta = beta.min(tt_value),
            }
            if alpha >= beta {
                return tt_value;
            }
        }
        ///////////////////////////////////////////////////////////////////
        // Compute the static eval once per node and track it per ply, for
        // future "improving" checks against the eval two plies ago. The
        // eval is meaningless while in check, so store -MATE there.
        ///////////////////////////////////////////////////////////////////
        let static_eval = if in_check { -i32::MATE } else { board.eval() };
        self.eval_stack[ply] = static_eval;

        ///////////////////////////////////////////////////////////////////
        // Reverse Futility Pruning
        ///////////////////////////////////////////////////////////////////
        if Self::can_apply_rfp(depth, in_check, is_pv, beta, excluded_move)
            && static_eval - Self::rfp_margin(depth) >= beta
        {
            return static_eval;
        }

        ///////////////////////////////////////////////////////////////////
        // Null move pruning.
        ///////////////////////////////////////////////////////////////////
        if Self::can_apply_null(
            board,
            depth,
            beta,
            static_eval,
            in_check,
            is_pv,
            excluded_move,
        ) {
            let r = Self::null_reduction(depth);
            board.push_null();
            let value = -self.search(board, depth - r - 1, -beta, -beta + 1, ply);
            board.pop_null();
            // The null-move search runs at this same ply and overwrites our
            // stack entry with the opponent-side eval, so restore it.
            self.eval_stack[ply] = static_eval;
            if self.timer.is_stopped() {
                return 0;
            }
            if value >= beta {
                return value;
            }
        }

        if Self::can_apply_iid(tt_entry, depth) {
            depth -= Self::IID_DEPTH_REDUCTION;
        }

        ///////////////////////////////////////////////////////////////////
        // Generate moves, score, and begin searching
        // recursively.
        ///////////////////////////////////////////////////////////////////
        let mut tt_flag = Bound::Upper;
        let mut best_move = None;
        let mut best_value = -i32::MATE;
        let mut quiets_tried = ArrayVec::<Move, MAX_MOVES>::new();

        let mut moves = MoveList::from::<false>(board);
        let sorter = self.scorer.create_sorter::<false>(
            &mut moves,
            board,
            ply,
            tt_entry.and_then(|entry| entry.best_move()),
        );

        for (idx, m) in sorter.enumerate() {
            if Some(m) == excluded_move {
                continue;
            }

            let extension = tt_entry
                .filter(|&entry| Self::can_singular_extend(entry, m, depth, excluded_move))
                .map_or(0, |entry| {
                    let target = entry.value() - (2 * depth as i32);
                    self.excluded_moves[ply] = Some(m);
                    let extension =
                        if self.search(board, (depth - 1) / 2, target - 1, target, ply) < target {
                            1
                        } else {
                            0
                        };
                    self.excluded_moves[ply] = None;
                    extension
                });

            ///////////////////////////////////////////////////////////////////
            // Make move and deepen search via principal variation search.
            ///////////////////////////////////////////////////////////////////
            board.push(m);

            if depth > 1 {
                self.tt.prefetch(board);
            }

            let mut value;
            if idx == 0 {
                value = -self.search(board, depth + extension - 1, -beta, -alpha, ply + 1);
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
                    value = -self.search(
                        board,
                        depth + extension - reduction - 1,
                        -alpha - 1,
                        -alpha,
                        ply + 1,
                    );
                    if value > alpha {
                        value = -self.search(
                            board,
                            depth + extension - reduction - 1,
                            -beta,
                            -alpha,
                            ply + 1,
                        );
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

            if self.timer.is_stopped() {
                return 0;
            }

            ///////////////////////////////////////////////////////////////////
            // Re-bound, check for cutoffs, and add killers and history.
            ///////////////////////////////////////////////////////////////////
            if value > best_value {
                best_value = value;

                if value > alpha {
                    best_move = Some(m);
                    if is_pv {
                        self.update_pv(m, ply);
                    }

                    if value >= beta {
                        if m.is_quiet() {
                            self.scorer.add_killer(m, ply);
                            self.scorer.add_history(m, board.ctm(), depth);
                            // Penalize the quiets searched before the cutoff
                            // move so they sort lower in future nodes.
                            for &q in &quiets_tried {
                                self.scorer.sub_history(q, board.ctm(), depth);
                            }
                            if let Some(p_move) = board.peek() {
                                self.scorer.add_counter(p_move, m);
                            }
                        }
                        tt_flag = Bound::Lower;
                        break;
                    }
                    tt_flag = Bound::Exact;
                    alpha = value;
                }
            }

            if m.is_quiet() {
                quiets_tried.push(m);
            }
        }

        ///////////////////////////////////////////////////////////////////
        // Checkmate and stalemate check.
        ///////////////////////////////////////////////////////////////////
        if moves.is_empty() && excluded_move.is_none() {
            if in_check {
                best_value = -mate_value;
            } else {
                best_value = 0;
            }
        }

        if !self.timer.is_stopped() && excluded_move.is_none() {
            best_move = best_move
                .or_else(|| self.tt.get(board, ply).and_then(|entry| entry.best_move()))
                .or_else(|| moves.into_iter().next().copied());

            self.tt
                .insert(board, depth, best_value, best_move, tt_flag, ply);
        }
        best_value
    }

    fn q_search(&mut self, board: &mut Board, mut alpha: i32, mut beta: i32, ply: usize) -> i32 {
        if self.timer.stop_check() {
            return 0;
        }

        if board.is_draw() {
            return 0;
        }

        self.sel_depth = self.sel_depth.max(ply);

        let tt_entry = self.tt.get(board, ply);
        if let Some(tt_entry) = tt_entry {
            let tt_value = tt_entry.value();

            match tt_entry.bound() {
                Bound::Exact => return tt_value,
                Bound::Lower => alpha = alpha.max(tt_value),
                Bound::Upper => beta = beta.min(tt_value),
            }
            if alpha >= beta {
                return tt_value;
            }
        }

        let eval = board.eval();

        if eval >= beta {
            self.tt.insert(board, 0, eval, None, Bound::Lower, ply);
            return eval;
        }
        alpha = alpha.max(eval);

        let mut moves = MoveList::from::<true>(board);
        let sorter = self.scorer.create_sorter::<true>(
            &mut moves,
            board,
            ply,
            tt_entry.and_then(|entry| entry.best_move()),
        );

        let mut tt_flag = Bound::Upper;
        let mut best_move = None;
        let mut best_value = eval;

        for m in sorter {
            if !MoveScorer::see(board, m, 0) {
                continue;
            }

            board.push(m);
            let value = -self.q_search(board, -beta, -alpha, ply + 1);
            board.pop();

            if self.timer.is_stopped() {
                return 0;
            }

            if value > best_value {
                best_value = value;

                if value > alpha {
                    best_move = Some(m);
                    if value >= beta {
                        tt_flag = Bound::Lower;
                        break;
                    }
                    tt_flag = Bound::Exact;
                    alpha = value;
                }
            }
        }

        if !self.timer.is_stopped() {
            self.tt
                .insert(board, 0, best_value, best_move, tt_flag, ply);
        }

        best_value
    }

    fn can_apply_null(
        board: &Board,
        depth: i8,
        beta: i32,
        static_eval: i32,
        in_check: bool,
        is_pv: bool,
        excluded_move: Option<Move>,
    ) -> bool {
        !is_pv
            && !in_check
            && board.peek().is_some()
            && depth >= Self::NULL_MIN_DEPTH
            && board.has_non_pawn_material()
            && static_eval >= beta
            && !beta.is_checkmate()
            && excluded_move.is_none()
    }

    fn can_apply_iid(tt_entry: Option<TTEntry>, depth: i8) -> bool {
        depth >= Self::IID_MIN_DEPTH && tt_entry.is_none_or(|entry| entry.best_move().is_none())
    }

    fn can_apply_rfp(
        depth: i8,
        in_check: bool,
        is_pv: bool,
        beta: i32,
        excluded_move: Option<Move>,
    ) -> bool {
        depth <= Self::RFP_MAX_DEPTH
            && !in_check
            && !is_pv
            && !beta.is_checkmate()
            && excluded_move.is_none()
    }

    fn can_apply_lmr(m: Move, depth: i8, move_index: usize) -> bool {
        depth >= Self::LMR_MIN_DEPTH && move_index >= Self::LMR_MOVE_WO_REDUCTION && m.is_quiet()
    }

    fn can_singular_extend(
        entry: TTEntry,
        m: Move,
        depth: i8,
        excluded_move: Option<Move>,
    ) -> bool {
        entry.best_move() == Some(m)
            && depth >= Self::SING_EXTEND_MIN_DEPTH
            && !entry.value().is_checkmate()
            && excluded_move.is_none()
            && entry.depth() + Self::SING_EXTEND_DEPTH_MARGIN >= depth
            && entry.bound() != Bound::Upper
    }

    fn null_reduction(depth: i8) -> i8 {
        // Idea of dividing in null move depth taken from Cosette
        Self::NULL_MIN_DEPTH_REDUCTION + (depth - Self::NULL_MIN_DEPTH) / Self::NULL_DEPTH_DIVIDER
    }

    fn rfp_margin(depth: i8) -> i32 {
        Self::RFP_MARGIN_MULTIPLIER * (depth as i32)
    }

    fn late_move_reduction(depth: i8, move_index: usize) -> i8 {
        // LMR table idea from Ethereal
        static LMR_TABLE: LazyLock<[[i8; 64]; 64]> = LazyLock::new(|| {
            let mut lmr_table = [[0; 64]; 64];
            for (depth, row) in lmr_table.iter_mut().enumerate().skip(1) {
                for (move_number, reduction) in row.iter_mut().enumerate().skip(1) {
                    *reduction = (Search::LMR_BASE_REDUCTION
                        + (depth as f32).ln() * (move_number as f32).ln()
                            / Search::LMR_MOVE_DIVIDER) as i8;
                }
            }
            lmr_table
        });

        LMR_TABLE[depth.min(63) as usize][move_index.min(63)]
    }

    fn update_pv(&mut self, m: Move, ply: usize) {
        let (before, after) = self.pv_table.split_at_mut(ply + 1);

        let pv = &mut before[ply];
        pv.clear();
        pv.push(m);

        if let Some(next_pv) = after.first() {
            pv.extend(next_pv);
        }

        after.iter_mut().for_each(|line| line.clear());
    }

    pub fn pv_wdl(board: &mut Board, pv: &[Move]) -> Option<[f32; 3]> {
        for &pv_move in pv {
            board.push(pv_move);
        }
        let mut leaf_ply = pv.len();
        while leaf_ply > 0 && (!pv[leaf_ply - 1].is_quiet() || board.in_check()) {
            board.pop();
            leaf_ply -= 1;
        }
        if leaf_ply == 0 {
            return None;
        }
        let mut wdl = board.wdl();
        for _ in 0..leaf_ply {
            board.pop();
        }
        // The head reports for the side to move at the leaf; flip back to
        // the root's perspective after an odd number of plies.
        if leaf_ply % 2 == 1 {
            wdl.reverse();
        }
        Some(wdl)
    }

    fn print_info(&self, board: &mut Board, depth: i8, line: &RootMove, multipv: Option<usize>) {
        let m = line.m;
        let value = line.value;
        let pv = &line.pv;

        let score_str = if value.is_checkmate() {
            let mate_value = (i32::MATE - value.abs() + 1) * value.signum() / 2;
            format!("mate {mate_value}")
        } else {
            format!("cp {value}")
        };

        let wdl_str = if self.show_wdl {
            // A proven mate overrides the network's prior.
            let [loss, draw, win] = if value.is_checkmate() {
                if value > 0 {
                    [0.0, 0.0, 1.0]
                } else {
                    [1.0, 0.0, 0.0]
                }
            } else {
                // The root read is fine as display cosmetics.
                Self::pv_wdl(board, pv).unwrap_or_else(|| board.wdl())
            };
            let per_mille = |p: f32| (p * 1000.0).round() as i32;
            format!(
                " wdl {} {} {}",
                per_mille(win),
                per_mille(draw),
                per_mille(loss)
            )
        } else {
            String::new()
        };

        let elapsed = self.timer.elapsed();
        let nodes = self.timer.nodes();
        let hashfull = self.tt.hashfull();
        let sel_depth = line.sel_depth;
        let time = elapsed.as_millis();
        let nps = (nodes as f64 / elapsed.as_secs_f64()) as u64;
        let multipv_str = multipv.map_or(String::new(), |n| format!(" multipv {n}"));
        let pv_str = pv
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<String>>()
            .join(" ");

        println!(
            "info currmove {m} depth {depth} seldepth {sel_depth}{multipv_str} time {time} score {score_str}{wdl_str} nodes {nodes} nps {nps} hashfull {hashfull} pv {pv_str}"
        );
    }

    fn print_currmovenumber(depth: i8, m: Move, idx: usize) {
        println!("info depth {depth} currmove {m} currmovenumber {}", idx + 1)
    }
}

impl Search<'_> {
    const PRINT_CURRMOVENUMBER_TIME: Duration = Duration::from_millis(3000);
    const RFP_MAX_DEPTH: i8 = 9;
    const RFP_MARGIN_MULTIPLIER: i32 = 63;
    const ASPIRATION_WINDOW: i32 = 61;
    const ASPIRATION_MIN_DEPTH: i8 = 4;
    const NULL_MIN_DEPTH: i8 = 2;
    const NULL_MIN_DEPTH_REDUCTION: i8 = 1;
    const NULL_DEPTH_DIVIDER: i8 = 2;
    const IID_MIN_DEPTH: i8 = 4;
    const IID_DEPTH_REDUCTION: i8 = 1;
    const LMR_MOVE_WO_REDUCTION: usize = 3;
    const LMR_MIN_DEPTH: i8 = 2;
    const LMR_BASE_REDUCTION: f32 = 0.11;
    const LMR_MOVE_DIVIDER: f32 = 1.56;
    const SING_EXTEND_MIN_DEPTH: i8 = 4;
    const SING_EXTEND_DEPTH_MARGIN: i8 = 2;
}

#[cfg(feature = "datagen")]
impl Search<'_> {
    // Like go, but with a soft node cap instead of a clock and the ranked
    // lines as the result.
    pub fn go_datagen(&mut self, board: &mut Board, soft_nodes: u64) -> Vec<RootMove> {
        let mut moves = MoveList::from::<false>(board);
        if moves.is_empty() {
            return Vec::new();
        }

        let hash_move = self.tt.get(board, 0).and_then(|entry| entry.best_move());
        let mut root_moves = self
            .scorer
            .create_sorter::<false>(&mut moves, board, 0, hash_move)
            .map(RootMove::new)
            .collect::<Vec<_>>();

        let multi_pv = self.multi_pv.min(root_moves.len());

        for depth in 1..i8::MAX {
            for pv_idx in 0..multi_pv {
                let (value, bound) = self.aspiration(board, depth, &mut root_moves[pv_idx..]);
                if pv_idx == 0 {
                    self.tt
                        .insert(board, depth, value, Some(root_moves[0].m), bound, 0);
                }
            }
            root_moves[..multi_pv].sort_by_key(|line| std::cmp::Reverse(line.value));

            if root_moves[0].value.is_checkmate() || self.timer.nodes() >= soft_nodes {
                break;
            }
        }

        root_moves.truncate(multi_pv);
        root_moves
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}
