use super::attacks;
use super::bitboard::*;
use super::castling::*;
use super::moov::*;
use super::move_list::*;
use super::nnue::*;
use super::piece::*;
use super::square::*;
use super::traits::*;
use super::zobrist::*;
use regex::Regex;
use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

#[derive(Clone)]
pub struct Board {
    board: SQMap<Option<Piece>>,
    piece_type_bb: PieceTypeMap<Bitboard>,
    color_bb: ColorMap<Bitboard>,
    history: [HistoryEntry; Self::N_HISTORIES],
    ctm: Color,
    ply: usize,
    material_hash: u64,
    network: Network,
}

impl Board {
    pub fn new() -> Self {
        Self::STARTING_FEN.parse().unwrap()
    }

    pub fn reset(&mut self) {
        self.set_fen(Self::STARTING_FEN).unwrap();
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn piece_at(&self, sq: SQ) -> Option<Piece> {
        self.board[sq]
    }

    pub fn piece_type_at(&self, sq: SQ) -> Option<PieceType> {
        self.board[sq].map(|pc| pc.type_of())
    }

    #[inline(always)]
    fn set_piece_at(&mut self, pc: Piece, sq: SQ) {
        self.network.activate(pc, sq);
        self.material_hash ^= ZOBRIST.update_hash(pc, sq);

        let bb = sq.bb();
        self.board[sq] = Some(pc);
        self.color_bb[pc.color_of()] |= bb;
        self.piece_type_bb[pc.type_of()] |= bb;
    }

    #[inline(always)]
    fn remove_piece(&mut self, sq: SQ) -> Option<Piece> {
        let pc = self.board[sq]?;

        self.network.deactivate(pc, sq);
        self.material_hash ^= ZOBRIST.update_hash(pc, sq);

        let bb_mask = !sq.bb();
        self.color_bb[pc.color_of()] &= bb_mask;
        self.piece_type_bb[pc.type_of()] &= bb_mask;
        self.board[sq] = None;

        Some(pc)
    }

    #[inline(always)]
    fn move_piece_quiet(&mut self, from_sq: SQ, to_sq: SQ) {
        let pc = self.board[from_sq].expect("Tried to move a piece off an empty square");

        self.network.move_piece_quiet(pc, from_sq, to_sq);
        self.material_hash ^= ZOBRIST.move_hash(pc, from_sq, to_sq);

        let mask = from_sq.bb() | to_sq.bb();
        self.color_bb[pc.color_of()] ^= mask;
        self.piece_type_bb[pc.type_of()] ^= mask;
        self.board[to_sq] = Some(pc);
        self.board[from_sq] = None;
    }

    pub fn eval(&self) -> i32 {
        self.network.eval(self.ctm)
    }

    pub fn wdl(&self) -> [f32; 3] {
        self.network.wdl(self.ctm)
    }

    fn refresh_network_if_needed(&mut self, color: Color) {
        let king_bb = self.bitboard_of(color, PieceType::King);

        let ksq_rel = king_bb.lsb().relative(color);
        if self.network.needs_refresh(color, ksq_rel) {
            let mut pieces = PieceMap::default();
            for pc in Piece::iter() {
                pieces[pc] = self.bitboard_of_pc(pc);
            }
            self.network.refresh(color, ksq_rel, &pieces);
        }
    }

    pub fn bitboard_of(&self, c: Color, pt: PieceType) -> Bitboard {
        self.piece_type_bb[pt] & self.color_bb[c]
    }

    pub fn bitboard_of_pc(&self, pc: Piece) -> Bitboard {
        self.piece_type_bb[pc.type_of()] & self.color_bb[pc.color_of()]
    }

    pub fn bitboard_of_pt(&self, pt: PieceType) -> Bitboard {
        self.piece_type_bb[pt]
    }

    pub fn diagonal_sliders(&self) -> Bitboard {
        self.bitboard_of_pt(PieceType::Bishop) | self.bitboard_of_pt(PieceType::Queen)
    }

    pub fn orthogonal_sliders(&self) -> Bitboard {
        self.bitboard_of_pt(PieceType::Rook) | self.bitboard_of_pt(PieceType::Queen)
    }

    pub fn diagonal_sliders_c(&self, color: Color) -> Bitboard {
        self.bitboard_of(color, PieceType::Bishop) | self.bitboard_of(color, PieceType::Queen)
    }

    pub fn orthogonal_sliders_c(&self, color: Color) -> Bitboard {
        self.bitboard_of(color, PieceType::Rook) | self.bitboard_of(color, PieceType::Queen)
    }

    pub fn all_pieces(&self) -> Bitboard {
        self.color_bb[Color::White] | self.color_bb[Color::Black]
    }

    pub fn all_pieces_c(&self, color: Color) -> Bitboard {
        self.color_bb[color]
    }

    pub fn attackers(&self, sq: SQ, occ: Bitboard) -> Bitboard {
        (self.bitboard_of(Color::White, PieceType::Pawn)
            & attacks::pawn_attacks_sq(sq, Color::Black))
            | (self.bitboard_of(Color::Black, PieceType::Pawn)
                & attacks::pawn_attacks_sq(sq, Color::White))
            | (self.bitboard_of_pt(PieceType::Knight) & attacks::knight_attacks(sq))
            | (self.diagonal_sliders() & attacks::bishop_attacks(sq, occ))
            | (self.orthogonal_sliders() & attacks::rook_attacks(sq, occ))
    }

    pub fn attackers_from_c(&self, sq: SQ, occ: Bitboard, color: Color) -> Bitboard {
        (self.bitboard_of(color, PieceType::Pawn) & attacks::pawn_attacks_sq(sq, !color))
            | (self.bitboard_of(color, PieceType::Knight) & attacks::knight_attacks(sq))
            | (self.diagonal_sliders_c(color) & attacks::bishop_attacks(sq, occ))
            | (self.orthogonal_sliders_c(color) & attacks::rook_attacks(sq, occ))
    }

    pub fn is_attacked(&self, sq: SQ) -> bool {
        let us = self.ctm;
        let them = !self.ctm;

        if attacks::knight_attacks(sq) & self.bitboard_of(them, PieceType::Knight) != Bitboard::ZERO
        {
            return true;
        }

        if attacks::pawn_attacks_sq(sq, us) & self.bitboard_of(them, PieceType::Pawn)
            != Bitboard::ZERO
        {
            return true;
        }

        let all = self.all_pieces();
        if attacks::rook_attacks(sq, all) & self.orthogonal_sliders_c(them) != Bitboard::ZERO {
            return true;
        }

        if attacks::bishop_attacks(sq, all) & self.diagonal_sliders_c(them) != Bitboard::ZERO {
            return true;
        }
        false
    }

    pub fn in_check(&self) -> bool {
        self.is_attacked(self.bitboard_of(self.ctm, PieceType::King).lsb())
    }

    pub fn peek(&self) -> Option<Move> {
        self.history[self.ply].moov
    }

    fn is_insufficient_material(&self) -> bool {
        match self.all_pieces().pop_count() {
            2 => true,
            3 => {
                self.bitboard_of_pt(PieceType::Rook)
                    | self.bitboard_of_pt(PieceType::Queen)
                    | self.bitboard_of_pt(PieceType::Pawn)
                    == Bitboard::ZERO
            }
            _ => false,
        }
    }

    fn is_fifty(&self) -> bool {
        self.history[self.ply].half_move_counter >= 100
    }

    fn is_repetition(&self) -> bool {
        let current = &self.history[self.ply];
        let lookback = current.plies_from_null.min(current.half_move_counter) as usize;

        self.history[self.ply - lookback..self.ply]
            .iter()
            .rev()
            .skip(1)
            .step_by(2)
            .any(|entry| {
                entry.material_hash == self.material_hash && entry.rights == current.rights
            })
    }

    pub fn is_draw(&self) -> bool {
        self.is_fifty() || self.is_insufficient_material() || self.is_repetition()
    }

    pub fn has_non_pawn_material(&self) -> bool {
        self.bitboard_of(self.ctm, PieceType::Pawn) | self.bitboard_of(self.ctm, PieceType::King)
            != self.all_pieces_c(self.ctm)
    }

    pub fn push_null(&mut self) {
        self.ply += 1;

        self.history[self.ply] = HistoryEntry {
            rights: self.history[self.ply - 1].rights,
            half_move_counter: self.history[self.ply - 1].half_move_counter + 1,
            plies_from_null: 0,
            moov: None,
            captured: None,
            epsq: None,
            material_hash: self.history[self.ply - 1].material_hash,
        };

        self.ctm = !self.ctm;
    }

    pub fn pop_null(&mut self) {
        self.ply -= 1;
        self.ctm = !self.ctm;
    }

    // The NNUE stack is copy-make: the piece helpers write to the current
    // network ply unconditionally, and in pop() those writes land on the ply
    // that network.pop() then discards.
    pub fn push(&mut self, m: Move) {
        let mut half_move_counter = self.history[self.ply].half_move_counter + 1;
        let mut captured = None;
        let mut epsq = None;
        let (from_sq, to_sq) = m.squares();
        self.ply += 1;
        self.network.push();

        if self.piece_type_at(from_sq) == Some(PieceType::Pawn) {
            half_move_counter = 0;
        }

        match m.flags() {
            MoveFlags::Quiet => {
                self.move_piece_quiet(from_sq, to_sq);
            }
            MoveFlags::DoublePush => {
                self.move_piece_quiet(from_sq, to_sq);
                epsq = Some(from_sq + Direction::North.relative(self.ctm));
            }
            MoveFlags::OO => {
                self.move_piece_quiet(SQ::E1.relative(self.ctm), SQ::G1.relative(self.ctm));
                self.move_piece_quiet(SQ::H1.relative(self.ctm), SQ::F1.relative(self.ctm));
            }
            MoveFlags::OOO => {
                self.move_piece_quiet(SQ::E1.relative(self.ctm), SQ::C1.relative(self.ctm));
                self.move_piece_quiet(SQ::A1.relative(self.ctm), SQ::D1.relative(self.ctm));
            }
            MoveFlags::EnPassant => {
                self.move_piece_quiet(from_sq, to_sq);
                self.remove_piece(to_sq + Direction::South.relative(self.ctm));
            }
            MoveFlags::Capture => {
                captured = self.piece_at(to_sq);
                half_move_counter = 0;
                self.remove_piece(to_sq);
                self.move_piece_quiet(from_sq, to_sq);
            }
            // Promotions:
            _ => {
                if m.is_capture() {
                    captured = self.remove_piece(to_sq);
                }
                self.remove_piece(from_sq);
                self.set_piece_at(
                    Piece::make_piece(
                        self.ctm,
                        m.promotion()
                            .expect("Tried to set a promotion piece for a non-promotion move."),
                    ),
                    to_sq,
                );
            }
        };

        self.refresh_network_if_needed(self.ctm);

        self.history[self.ply] = HistoryEntry {
            rights: self.history[self.ply - 1]
                .rights
                .without(CastlingRights::killed(from_sq) | CastlingRights::killed(to_sq)),
            moov: Some(m),
            plies_from_null: self.history[self.ply - 1].plies_from_null + 1,
            material_hash: self.material_hash,
            half_move_counter,
            captured,
            epsq,
        };
        self.ctm = !self.ctm;
    }

    pub fn pop(&mut self) -> Option<Move> {
        self.ctm = !self.ctm;

        let m = self.history[self.ply].moov?;
        let (from_sq, to_sq) = m.squares();

        match m.flags() {
            MoveFlags::Quiet => {
                self.move_piece_quiet(to_sq, from_sq);
            }
            MoveFlags::DoublePush => {
                self.move_piece_quiet(to_sq, from_sq);
            }
            MoveFlags::OO => {
                self.move_piece_quiet(SQ::G1.relative(self.ctm), SQ::E1.relative(self.ctm));
                self.move_piece_quiet(SQ::F1.relative(self.ctm), SQ::H1.relative(self.ctm));
            }
            MoveFlags::OOO => {
                self.move_piece_quiet(SQ::C1.relative(self.ctm), SQ::E1.relative(self.ctm));
                self.move_piece_quiet(SQ::D1.relative(self.ctm), SQ::A1.relative(self.ctm));
            }
            MoveFlags::EnPassant => {
                self.move_piece_quiet(to_sq, from_sq);
                self.set_piece_at(
                    Piece::make_piece(!self.ctm, PieceType::Pawn),
                    to_sq + Direction::South.relative(self.ctm),
                );
            }
            MoveFlags::PrKnight | MoveFlags::PrBishop | MoveFlags::PrRook | MoveFlags::PrQueen => {
                self.remove_piece(to_sq);
                self.set_piece_at(Piece::make_piece(self.ctm, PieceType::Pawn), from_sq);
            }
            MoveFlags::PcKnight | MoveFlags::PcBishop | MoveFlags::PcRook | MoveFlags::PcQueen => {
                self.remove_piece(to_sq);
                self.set_piece_at(Piece::make_piece(self.ctm, PieceType::Pawn), from_sq);
                self.set_piece_at(
                    self.history[self.ply]
                        .captured
                        .expect("Tried to revert a capture move with no capture."),
                    to_sq,
                );
            }
            MoveFlags::Capture => {
                self.move_piece_quiet(to_sq, from_sq);
                self.set_piece_at(
                    self.history[self.ply]
                        .captured
                        .expect("Tried to revert a capture move with no capture."),
                    to_sq,
                );
            }
        }
        self.ply -= 1;
        self.network.pop();
        Some(m)
    }

    pub fn generate_legal_moves<const QUIESCENCE: bool>(&self, moves: &mut MoveList) {
        let us = self.ctm;
        let them = !self.ctm;

        let us_bb = self.all_pieces_c(us);
        let them_bb = self.all_pieces_c(them);
        let all = us_bb | them_bb;

        let our_king = self.bitboard_of(us, PieceType::King).lsb();

        let their_king = self.bitboard_of(them, PieceType::King).lsb();

        let our_diag_sliders = self.diagonal_sliders_c(us);
        let their_diag_sliders = self.diagonal_sliders_c(them);
        let our_orth_sliders = self.orthogonal_sliders_c(us);
        let their_orth_sliders = self.orthogonal_sliders_c(them);

        ///////////////////////////////////////////////////////////////////
        // Danger squares for the king
        ///////////////////////////////////////////////////////////////////
        let mut danger = Bitboard::ZERO;

        ///////////////////////////////////////////////////////////////////
        // Add each enemy attack to the danger bitboard
        ///////////////////////////////////////////////////////////////////
        danger |= attacks::pawn_attacks_bb(self.bitboard_of(them, PieceType::Pawn), them)
            | attacks::king_attacks(their_king);

        danger |= self
            .bitboard_of(them, PieceType::Knight)
            .map(attacks::knight_attacks)
            .fold(Bitboard::ZERO, |a, b| a | b);

        danger |= their_diag_sliders
            .map(|sq| attacks::bishop_attacks(sq, all ^ our_king.bb()))
            .fold(Bitboard::ZERO, |a, b| a | b);

        danger |= their_orth_sliders
            .map(|sq| attacks::rook_attacks(sq, all ^ our_king.bb()))
            .fold(Bitboard::ZERO, |a, b| a | b);

        ///////////////////////////////////////////////////////////////////
        // The king can move to any square that isn't attacked or occupied
        // by one of our pieces.
        ///////////////////////////////////////////////////////////////////

        let king_attacks = attacks::king_attacks(our_king) & !(us_bb | danger);

        if !QUIESCENCE {
            moves.make_q(our_king, king_attacks & !them_bb);
        }
        moves.make_c(our_king, king_attacks & them_bb);

        ///////////////////////////////////////////////////////////////////
        // The capture mask consists of destination squares containing enemy
        // pieces that must be captured because they are checking the king.
        ///////////////////////////////////////////////////////////////////
        let capture_mask;

        ///////////////////////////////////////////////////////////////////
        // The quiet mask consists of squares where pieces must be moved
        // to block an attack checking the king.
        ///////////////////////////////////////////////////////////////////
        let quiet_mask;

        ///////////////////////////////////////////////////////////////////
        // Checkers are identified by projecting attacks from the king
        // square and then intersecting them with the enemy bitboard of the
        // respective piece.
        ///////////////////////////////////////////////////////////////////
        let mut checkers = (attacks::knight_attacks(our_king)
            & self.bitboard_of(them, PieceType::Knight))
            | (attacks::pawn_attacks_sq(our_king, us) & self.bitboard_of(them, PieceType::Pawn));

        ///////////////////////////////////////////////////////////////////
        // Candidates are potential slider checkers and pinners.
        ///////////////////////////////////////////////////////////////////
        let candidates = (attacks::rook_attacks(our_king, them_bb) & their_orth_sliders)
            | (attacks::bishop_attacks(our_king, them_bb) & their_diag_sliders);

        let mut pinned = Bitboard::ZERO;

        for sq in candidates {
            let potentially_pinned = Bitboard::between(our_king, sq) & us_bb;

            ///////////////////////////////////////////////////////////////////
            // Do the squares between an enemy slider and our king contain any
            // pieces? If yes, that piece is pinned. Otherwise, we are checked.
            ///////////////////////////////////////////////////////////////////
            if potentially_pinned == Bitboard::ZERO {
                checkers ^= sq.bb();
            } else if potentially_pinned.is_single() {
                pinned ^= potentially_pinned;
            }
        }

        let not_pinned = !pinned;

        match checkers.pop_count() {
            2 => {
                ///////////////////////////////////////////////////////////////////
                // If we're in a double check, we have to move the king. We've already
                // generated those moves, so just return.
                ///////////////////////////////////////////////////////////////////
                return;
            }
            1 => {
                let checker_square = checkers.lsb();
                let pt = self
                    .piece_type_at(checker_square)
                    .expect("Checker expected.");
                match pt {
                    PieceType::Pawn | PieceType::Knight => {
                        ///////////////////////////////////////////////////////////////////
                        // If the checkers is a pawn, we have to look out for ep moves
                        // that can capture it.
                        ///////////////////////////////////////////////////////////////////
                        if pt == PieceType::Pawn
                            && let Some(epsq) = self.history[self.ply].epsq
                            && checkers == epsq.bb().shift(Direction::South.relative(us))
                        {
                            let pawns = attacks::pawn_attacks_sq(epsq, them)
                                & self.bitboard_of(us, PieceType::Pawn)
                                & not_pinned;
                            for sq in pawns {
                                moves.push(Move::new(sq, epsq, MoveFlags::EnPassant));
                            }
                        }
                        let checker_attackers =
                            self.attackers_from_c(checker_square, all, us) & not_pinned;
                        for sq in checker_attackers {
                            if self.piece_type_at(sq) == Some(PieceType::Pawn)
                                && sq.rank().relative(us) == Rank::Seven
                            {
                                moves.make_promotions::<QUIESCENCE>(sq, checker_square, true);
                            } else {
                                moves.push(Move::new(sq, checker_square, MoveFlags::Capture));
                            }
                        }
                        return;
                    }
                    _ => {
                        ///////////////////////////////////////////////////////////////////
                        // We have to either capture the piece or block it, since it must be
                        // a slider.
                        ///////////////////////////////////////////////////////////////////
                        capture_mask = checkers;
                        quiet_mask = Bitboard::between(our_king, checker_square);
                    }
                }
            }
            _ => {
                ///////////////////////////////////////////////////////////////////
                // At this point, we can capture any enemy piece or play into any
                // quiet square.
                ///////////////////////////////////////////////////////////////////
                capture_mask = them_bb;
                quiet_mask = !all;

                self.push_ep_moves(moves, all, pinned, our_king);
                self.push_castling_moves::<QUIESCENCE>(moves, all, danger);
                self.push_pinned_moves::<QUIESCENCE>(
                    moves,
                    all,
                    pinned,
                    our_king,
                    quiet_mask,
                    capture_mask,
                );
            }
        }

        ///////////////////////////////////////////////////////////////////
        // Non-pinned moves from here
        ///////////////////////////////////////////////////////////////////
        for sq in self.bitboard_of(us, PieceType::Knight) & not_pinned {
            let knight_attacks = attacks::knight_attacks(sq);
            moves.make_c(sq, knight_attacks & capture_mask);
            if !QUIESCENCE {
                moves.make_q(sq, knight_attacks & quiet_mask);
            }
        }

        for sq in our_diag_sliders & not_pinned {
            let diag_attacks = attacks::bishop_attacks(sq, all);
            moves.make_c(sq, diag_attacks & capture_mask);
            if !QUIESCENCE {
                moves.make_q(sq, diag_attacks & quiet_mask);
            }
        }

        for sq in our_orth_sliders & not_pinned {
            let orth_attacks = attacks::rook_attacks(sq, all);
            moves.make_c(sq, orth_attacks & capture_mask);
            if !QUIESCENCE {
                moves.make_q(sq, orth_attacks & quiet_mask);
            }
        }

        self.push_pawn_moves::<QUIESCENCE>(moves, all, pinned, quiet_mask, capture_mask);
    }

    fn push_ep_moves(&self, moves: &mut MoveList, all: Bitboard, pinned: Bitboard, our_king: SQ) {
        ///////////////////////////////////////////////////////////////////
        // En passant, both for unpinned attackers (with the revealed-check
        // guard) and diagonally pinned ones. Never called while in check.
        ///////////////////////////////////////////////////////////////////
        let Some(epsq) = self.history[self.ply].epsq else {
            return;
        };

        let us = self.ctm;
        let them = !self.ctm;

        let epsq_attackers =
            attacks::pawn_attacks_sq(epsq, them) & self.bitboard_of(us, PieceType::Pawn);
        let unpinned_epsq_attackers = epsq_attackers & !pinned;
        for sq in unpinned_epsq_attackers {
            ///////////////////////////////////////////////////////////////////
            // From surge:
            // This piece of evil bit-fiddling magic prevents the infamous 'pseudo-pinned' e.p. case,
            // where the pawn is not directly pinned, but on moving the pawn and capturing the enemy pawn
            // e.p., a rook or queen attack to the king is revealed
            //
            //
            // nbqkbnr
            // ppp.pppp
            // ........
            // r..pP..K
            // ........
            // ........
            // PPPP.PPP
            // RNBQ.BNR
            //
            // Here, if white plays exd5 e.p., the black rook on a5 attacks the white king on h5
            ///////////////////////////////////////////////////////////////////
            let attacks = attacks::sliding_attacks(
                our_king,
                all ^ sq.bb() ^ epsq.bb().shift(Direction::South.relative(us)),
                our_king.rank().bb(),
            );

            if (attacks & self.orthogonal_sliders_c(them)) == Bitboard::ZERO {
                moves.push(Move::new(sq, epsq, MoveFlags::EnPassant));
            }
        }
        ///////////////////////////////////////////////////////////////////
        // Pinned pawns can only capture ep if they are pinned diagonally
        // and the ep square is in line with the king.
        ///////////////////////////////////////////////////////////////////
        let pinned_epsq_attackers = epsq_attackers & pinned & Bitboard::line(epsq, our_king);
        if pinned_epsq_attackers != Bitboard::ZERO {
            moves.push(Move::new(
                pinned_epsq_attackers.lsb(),
                epsq,
                MoveFlags::EnPassant,
            ));
        }
    }

    fn push_castling_moves<const QUIESCENCE: bool>(
        &self,
        moves: &mut MoveList,
        all: Bitboard,
        danger: Bitboard,
    ) {
        ///////////////////////////////////////////////////////////////////
        // Only castle if:
        // 1. Not in quiescence, since castling is never a capture.
        // 2. Neither the king nor rook have moved.
        // 3. The king is not in check (never called while in check).
        // 4. The relevant squares are not attacked.
        ///////////////////////////////////////////////////////////////////
        if QUIESCENCE {
            return;
        }

        let us = self.ctm;
        let rights = self.history[self.ply].rights;

        if rights.contains(CastlingRights::oo(us))
            && all & CastlingRights::oo_path(us) == Bitboard::ZERO
            && danger & CastlingRights::oo_king_path(us) == Bitboard::ZERO
        {
            moves.push(match us {
                Color::White => Move::new(SQ::E1, SQ::G1, MoveFlags::OO),
                Color::Black => Move::new(SQ::E8, SQ::G8, MoveFlags::OO),
            });
        }
        if rights.contains(CastlingRights::ooo(us))
            && all & CastlingRights::ooo_path(us) == Bitboard::ZERO
            && danger & CastlingRights::ooo_king_path(us) == Bitboard::ZERO
        {
            moves.push(match us {
                Color::White => Move::new(SQ::E1, SQ::C1, MoveFlags::OOO),
                Color::Black => Move::new(SQ::E8, SQ::C8, MoveFlags::OOO),
            });
        }
    }

    fn push_pinned_moves<const QUIESCENCE: bool>(
        &self,
        moves: &mut MoveList,
        all: Bitboard,
        pinned: Bitboard,
        our_king: SQ,
        quiet_mask: Bitboard,
        capture_mask: Bitboard,
    ) {
        ///////////////////////////////////////////////////////////////////
        // Pinned pieces may only move along the line between our king and
        // their pinner. Never called while in check.
        ///////////////////////////////////////////////////////////////////
        let us = self.ctm;
        let them_bb = self.all_pieces_c(!us);

        ///////////////////////////////////////////////////////////////////
        // For each pinned rook, bishop, or queen, only include attacks
        // that are aligned with our king. Pinned pawns are handled below,
        // and a pinned knight can never move at all.
        ///////////////////////////////////////////////////////////////////
        let pinned_pieces = pinned
            & !(self.bitboard_of(us, PieceType::Knight) | self.bitboard_of(us, PieceType::Pawn));
        for sq in pinned_pieces {
            let pt = self
                .piece_type_at(sq)
                .expect("Unexpected None for piece type.");
            let attacks_along_pin = attacks::attacks(pt, sq, all) & Bitboard::line(our_king, sq);
            if !QUIESCENCE {
                moves.make_q(sq, attacks_along_pin & quiet_mask);
            }
            moves.make_c(sq, attacks_along_pin & capture_mask);
        }

        ///////////////////////////////////////////////////////////////////
        // For each pinned pawn
        ///////////////////////////////////////////////////////////////////
        let pinned_pawns = pinned & self.bitboard_of(us, PieceType::Pawn);
        for sq in pinned_pawns {
            ///////////////////////////////////////////////////////////////////
            // Quiet promotions are impossible since the square in front of the
            // pawn will be occupied
            ///////////////////////////////////////////////////////////////////
            if sq.rank() == Rank::Seven.relative(us) {
                moves.make_pc(
                    sq,
                    attacks::pawn_attacks_sq(sq, us) & capture_mask & Bitboard::line(our_king, sq),
                );
            } else {
                moves.make_c(
                    sq,
                    attacks::pawn_attacks_sq(sq, us) & them_bb & Bitboard::line(sq, our_king),
                );

                ///////////////////////////////////////////////////////////////////
                // Single and double pawn pushes
                ///////////////////////////////////////////////////////////////////
                if !QUIESCENCE {
                    let single_pinned_pushes = sq.bb().shift(Direction::North.relative(us))
                        & !all
                        & Bitboard::line(our_king, sq);
                    let double_pinned_pushes = (single_pinned_pushes
                        & Rank::Three.relative(us).bb())
                    .shift(Direction::North.relative(us))
                        & !all
                        & Bitboard::line(our_king, sq);

                    moves.make_q(sq, single_pinned_pushes);
                    moves.make_dp(sq, double_pinned_pushes);
                }
            }
        }
    }

    fn push_pawn_moves<const QUIESCENCE: bool>(
        &self,
        moves: &mut MoveList,
        all: Bitboard,
        pinned: Bitboard,
        quiet_mask: Bitboard,
        capture_mask: Bitboard,
    ) {
        ///////////////////////////////////////////////////////////////////
        // Non-pinned pawn moves: pushes, captures, and promotions, all
        // filtered through the check-evasion masks.
        ///////////////////////////////////////////////////////////////////
        let us = self.ctm;
        let not_pinned = !pinned;

        let back_pawns =
            self.bitboard_of(us, PieceType::Pawn) & not_pinned & !Rank::Seven.relative(us).bb();
        let mut single_pushes = back_pawns.shift(Direction::North.relative(us)) & !all;
        let double_pushes = (single_pushes & Rank::Three.relative(us).bb())
            .shift(Direction::North.relative(us))
            & quiet_mask;

        single_pushes &= quiet_mask;

        if !QUIESCENCE {
            for sq in single_pushes {
                moves.push(Move::new(
                    sq - Direction::North.relative(us),
                    sq,
                    MoveFlags::Quiet,
                ));
            }

            for sq in double_pushes {
                moves.push(Move::new(
                    sq - Direction::NorthNorth.relative(us),
                    sq,
                    MoveFlags::DoublePush,
                ));
            }
        }

        for dir in [Direction::NorthWest, Direction::NorthEast] {
            let captures = back_pawns.shift(dir.relative(us)) & capture_mask;
            for sq in captures {
                moves.push(Move::new(sq - dir.relative(us), sq, MoveFlags::Capture));
            }
        }

        let seventh_rank_pawns =
            self.bitboard_of(us, PieceType::Pawn) & not_pinned & Rank::Seven.relative(us).bb();

        if seventh_rank_pawns != Bitboard::ZERO {
            let quiet_promotions =
                seventh_rank_pawns.shift(Direction::North.relative(us)) & quiet_mask;
            for sq in quiet_promotions {
                moves.make_promotions::<QUIESCENCE>(sq - Direction::North.relative(us), sq, false);
            }

            for dir in [Direction::NorthWest, Direction::NorthEast] {
                let promotion_captures = seventh_rank_pawns.shift(dir.relative(us)) & capture_mask;
                for sq in promotion_captures {
                    moves.make_promotions::<QUIESCENCE>(sq - dir.relative(us), sq, true);
                }
            }
        }
    }

    pub fn push_str(&mut self, move_str: &str) -> Result<(), &'static str> {
        let moves = MoveList::from::<false>(self);
        let m = moves
            .into_iter()
            .find(|m| m.to_string() == move_str)
            .ok_or("Invalid move.")?;

        self.push(*m);
        Ok(())
    }

    pub fn set_fen(&mut self, fen: &str) -> Result<(), &'static str> {
        self.clear();
        let fen = fen.trim();
        if !fen.is_ascii() || fen.lines().count() != 1 {
            return Err("FEN should be a single ASCII line.");
        }

        let re_captures = FEN_RE.captures(fen).ok_or("Invalid fen format.")?;

        let piece_placement = re_captures
            .name("piece_placement")
            .ok_or("Invalid piece placement.")?
            .as_str();
        let ctm = re_captures
            .name("active_color")
            .ok_or("Invalid color.")?
            .as_str();
        let castling = re_captures
            .name("castling")
            .ok_or("Invalid castling rights.")?
            .as_str();
        let en_passant_sq = re_captures.name("en_passant").map_or("-", |m| m.as_str());
        let halfmove_clock = re_captures.name("halfmove").map_or("0", |m| m.as_str());
        let fullmove_counter = re_captures.name("fullmove").map_or("1", |m| m.as_str());

        if piece_placement.split('/').count() != Rank::COUNT {
            return Err("Pieces Placement FEN should have 8 ranks.");
        }

        self.ctm = Color::try_from(ctm.parse::<char>().map_err(|_| "Invalid color.")?)?;

        self.ply = 2
            * (fullmove_counter
                .parse::<usize>()
                .map_err(|_| "Invalid full move counter.")?
                - 1);
        if self.ctm == Color::Black {
            self.ply += 1;
        }

        let ranks = piece_placement.split('/');
        for (rank_idx, rank_fen) in ranks.enumerate() {
            let mut idx = (7 - rank_idx) * 8;

            for ch in rank_fen.chars() {
                if let Some(digit) = ch.to_digit(10) {
                    if digit > 8 {
                        return Err("Invalid digit in position.");
                    }
                    idx += digit as usize;
                } else {
                    if idx > 63 {
                        return Err("Invalid square index in FEN.");
                    }
                    let sq = SQ::from_repr(idx as u8);
                    let pc = Piece::try_from(ch)?;
                    self.set_piece_at(pc, sq);
                    idx += 1;
                }
            }

            if idx != 64 - 8 * rank_idx {
                return Err("FEN rank does not fill expected number of squares.");
            }
        }

        self.refresh_network_if_needed(Color::White);
        self.refresh_network_if_needed(Color::Black);

        let epsq = (en_passant_sq != "-")
            .then(|| en_passant_sq.parse())
            .transpose()?;

        let half_move_counter = halfmove_clock
            .parse::<u16>()
            .map_err(|_| "Invalid half move counter.")?;

        self.history[self.ply] = HistoryEntry {
            rights: castling.parse()?,
            moov: None,
            material_hash: self.material_hash,
            plies_from_null: 0,
            captured: None,
            epsq,
            half_move_counter,
        };
        Ok(())
    }

    pub fn ctm(&self) -> Color {
        self.ctm
    }

    pub fn ply(&self) -> usize {
        self.ply
    }

    pub fn hash(&self) -> u64 {
        self.material_hash
            ^ ZOBRIST.castling_hash(self.history[self.ply].rights)
            ^ self.history[self.ply]
                .epsq
                .map_or(0, |sq| ZOBRIST.ep_hash(sq))
            ^ ZOBRIST.color_hash(self.ctm)
    }

    pub fn material_hash(&self) -> u64 {
        self.material_hash
    }

    pub fn fullmove_number(&self) -> usize {
        self.ply / 2 + 1
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            piece_type_bb: PieceTypeMap::default(),
            color_bb: ColorMap::default(),
            board: SQMap::default(),
            ctm: Color::White,
            ply: 0,
            material_hash: 0,
            network: Network::new(),
            history: [HistoryEntry::default(); Self::N_HISTORIES],
        }
    }
}

impl FromStr for Board {
    type Err = &'static str;

    fn from_str(fen: &str) -> Result<Self, Self::Err> {
        let mut board = Board::default();
        board.set_fen(fen)?;
        Ok(board)
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut board_str = String::new();
        for rank_idx in (0..=7).rev() {
            let rank = Rank::from_repr(rank_idx);
            let mut empty_squares = 0;
            for file_idx in 0..=7 {
                let file = File::from_repr(file_idx);
                let sq = SQ::encode(rank, file);
                match self.board[sq] {
                    Some(pc) => {
                        if empty_squares != 0 {
                            board_str.push_str(empty_squares.to_string().as_str());
                            empty_squares = 0;
                        }
                        board_str.push_str(pc.to_string().as_str());
                    }
                    None => {
                        empty_squares += 1;
                    }
                }
            }
            if empty_squares != 0 {
                board_str.push_str(empty_squares.to_string().as_str());
            }
            if rank != Rank::One {
                board_str.push('/');
            }
        }

        let epsq_str = self.history[self.ply]
            .epsq
            .map_or("-".to_string(), |epsq| epsq.to_string());

        write!(
            f,
            "{} {} {} {} {} {}",
            board_str,
            self.ctm,
            self.history[self.ply].rights,
            epsq_str,
            self.history[self.ply].half_move_counter,
            self.ply / 2 + 1,
        )
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::with_capacity(SQ::COUNT * 2 + 8);
        for rank_idx in (0..=7).rev() {
            let rank = Rank::from_repr(rank_idx);
            for file_idx in 0..=7 {
                let file = File::from_repr(file_idx);
                let sq = SQ::encode(rank, file);
                let pc_str = self
                    .piece_at(sq)
                    .map_or("-".to_string(), |pc| pc.to_string());
                s.push_str(&pc_str);
                s.push(' ');
                if sq.file() == File::H {
                    s.push('\n');
                }
            }
        }
        write!(f, "{s}")
    }
}

impl Board {
    const N_HISTORIES: usize = 1024;
    const STARTING_FEN: &'static str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
}

static FEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)^
                (?P<piece_placement>[KQRBNPkqrbnp1-8/]+)\s+
                (?P<active_color>[wb])\s+
                (?P<castling>[KQkq\-]+)\s+
                (?P<en_passant>[a-h1-8\-]+)
                (?:\s+(?P<halfmove>\d+))?
                (?:\s+(?P<fullmove>\d+))?
            $",
    )
    .expect("Failed to compile fen regex.")
});

#[derive(Clone, Copy, Debug, Default)]
pub struct HistoryEntry {
    rights: CastlingRights,
    captured: Option<Piece>,
    epsq: Option<SQ>,
    moov: Option<Move>,
    material_hash: u64,
    half_move_counter: u16,
    plies_from_null: u16,
}

#[cfg(test)]
mod tests {
    use crate::board::*;

    // Walk every move sequence to the given depth, checking at each node
    // that the incrementally maintained accumulator gives the same eval as
    // one rebuilt from scratch via FEN. This is the safety net for the
    // mirror-refresh machinery: a missed or wrong refresh shows up as a
    // divergence at the first node whose king crossed the d/e boundary.
    fn walk_evals(board: &mut Board, depth: u8) {
        let mut fresh = Board::new();
        fresh.set_fen(&board.to_string()).expect("Roundtrip FEN.");
        assert_eq!(
            board.eval(),
            fresh.eval(),
            "incremental eval != from-scratch eval at {board}"
        );

        if depth == 0 {
            return;
        }

        let moves = MoveList::from::<false>(board);
        for &m in &moves {
            board.push(m);
            walk_evals(board, depth - 1);
            board.pop();
        }
    }

    #[test]
    fn nnue_incremental_consistency() {
        let positions = [
            // Startpos: both kings on the e-file, i.e. both perspectives
            // mirrored, castling in both directions available.
            (Board::STARTING_FEN, 2),
            // Kiwipete: all castling rights, promotions nearby, tactical.
            (
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                2,
            ),
            // En passant and kings on opposite wings.
            ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 3),
            // Promotions with captures on the back rank.
            (
                "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
                2,
            ),
            // Bare kings next to the mirror boundary: deep walk forces
            // many boundary crossings for both colors.
            ("8/3k4/8/8/8/8/4K3/8 w - - 0 1", 5),
        ];

        for (fen, depth) in positions {
            let mut board = Board::new();
            board.set_fen(fen).expect("Test FEN should be valid.");
            walk_evals(&mut board, depth);
        }
    }

    #[test]
    fn castling_rights_updates() {
        let rights = |board: &Board| board.to_string().split(' ').nth(2).unwrap().to_string();

        // Rook takes rook: one move kills a right on each side.
        let mut board = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"
            .parse::<Board>()
            .unwrap();
        board.push_str("a1a8").unwrap();
        assert_eq!(rights(&board), "Kk");
        board.pop();
        assert_eq!(rights(&board), "KQkq");

        // King moves kill both of their side's rights, castling included.
        board.push_str("e1g1").unwrap();
        assert_eq!(rights(&board), "kq");
        board.push_str("e8d8").unwrap();
        assert_eq!(rights(&board), "-");
        board.pop();

        // A rook move kills only its own wing.
        board.push_str("h8h1").unwrap();
        assert_eq!(rights(&board), "q");
    }

    #[test]
    fn repetition_needs_castling_rights() {
        let mut board = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"
            .parse::<Board>()
            .unwrap();
        for m in ["e1e2", "e8e7", "e2e1", "e7e8"] {
            board.push_str(m).unwrap();
        }
        assert_eq!(board.is_repetition(), false);

        for m in ["e1e2", "e8e7", "e2e1", "e7e8"] {
            board.push_str(m).unwrap();
        }

        assert_eq!(board.is_repetition(), true);
    }

    #[test]
    fn castling_rights_hashed() {
        let kiwipete = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w";
        let boards = ["KQkq", "KQ", "kq", "-"].map(|rights| {
            format!("{kiwipete} {rights} - 0 1")
                .parse::<Board>()
                .unwrap()
        });

        // Identical pieces, so any hash difference must come from the rights.
        for (i, a) in boards.iter().enumerate() {
            for b in &boards[i + 1..] {
                assert_eq!(a.material_hash(), b.material_hash());
                assert_ne!(a.hash(), b.hash());
            }
        }
    }

    #[test]
    fn threefold_repetition() {
        let mut board = Board::new();
        assert_eq!(board.is_repetition(), false);
        board.push_str("e2e4").unwrap();
        assert_eq!(board.is_repetition(), false);
        board.push_str("e7e5").unwrap();
        assert_eq!(board.is_repetition(), false);
        board.push_str("f1c4").unwrap();
        assert_eq!(board.is_repetition(), false);
        board.push_str("f8c5").unwrap();
        assert_eq!(board.is_repetition(), false);
        board.push_str("c4f1").unwrap();
        assert_eq!(board.is_repetition(), false);
        board.push_str("c5f8").unwrap();
        assert_eq!(board.is_repetition(), true);
    }
}
