use super::attacks;
use super::bitboard::*;
use super::moov::*;
use super::move_list::*;
use super::nnue::*;
use super::piece::*;
use super::square::*;
use super::types::*;
use super::zobrist::*;
use regex::Regex;
use std::fmt;
use std::sync::LazyLock;

#[derive(Clone)]
pub struct Board {
    board: SQMap<Option<Piece>>,
    piece_type_bb: PieceTypeMap<Bitboard>,
    color_bb: ColorMap<Bitboard>,
    history: [HistoryEntry; Self::N_HISTORIES],
    ctm: Color,
    ply: usize,
    hasher: Hasher,
    network: Network,
}

impl Board {
    pub fn new() -> Self {
        Self::try_from(Self::STARTING_FEN).unwrap()
    }

    pub fn reset(&mut self) {
        self.set_fen(Self::STARTING_FEN).unwrap();
    }

    pub fn clear(&mut self) {
        self.ply = 0;
        self.ctm = Color::White;
        self.history = [HistoryEntry::default(); Self::N_HISTORIES];

        self.color_bb = ColorMap::new([Bitboard::ZERO; Color::N_COLORS]);
        self.piece_type_bb = PieceTypeMap::new([Bitboard::ZERO; PieceType::N_PIECE_TYPES]);
        self.board = SQMap::new([None; SQ::N_SQUARES]);

        self.hasher.clear();
        self.network = Network::new();
    }

    pub fn piece_at(&self, sq: SQ) -> Option<Piece> {
        self.board[sq]
    }

    pub fn piece_type_at(&self, sq: SQ) -> Option<PieceType> {
        self.board[sq].map(|pc| pc.type_of())
    }

    pub fn set_piece_at(&mut self, pc: Piece, sq: SQ) {
        self.network.activate(pc, sq);
        self.hasher.update_piece(pc, sq);

        self.board[sq] = Some(pc);
        self.color_bb[pc.color_of()] |= sq.bb();
        self.piece_type_bb[pc.type_of()] |= sq.bb();
    }

    pub fn remove_piece(&mut self, sq: SQ) {
        let pc = self
            .piece_at(sq)
            .expect("Tried to remove a piece from an empty square.");

        self.network.deactivate(pc, sq);
        self.hasher.update_piece(pc, sq);

        self.piece_type_bb[pc.type_of()] &= !sq.bb();
        self.color_bb[pc.color_of()] &= !sq.bb();
        self.board[sq] = None;
    }

    pub fn move_piece_quiet(&mut self, from_sq: SQ, to_sq: SQ) {
        let pc = self
            .piece_at(from_sq)
            .expect("Tried to move a piece off of an empty square.");

        self.network.move_piece(pc, from_sq, to_sq);
        self.hasher.move_piece(pc, from_sq, to_sq);

        let mask = from_sq.bb() | to_sq.bb();
        self.piece_type_bb[pc.type_of()] ^= mask;
        self.color_bb[pc.color_of()] ^= mask;
        self.board[to_sq] = self.board[from_sq];
        self.board[from_sq] = None;
    }

    pub fn move_piece(&mut self, from_sq: SQ, to_sq: SQ) {
        self.remove_piece(to_sq);
        self.move_piece_quiet(from_sq, to_sq);
    }

    pub fn eval(&self) -> Value {
        self.network.eval(self.ctm)
    }

    pub fn bitboard_of(&self, c: Color, pt: PieceType) -> Bitboard {
        self.piece_type_bb[pt] & self.color_bb[c]
    }

    pub fn bitboard_of_pc(&self, pc: Piece) -> Bitboard {
        self.piece_type_bb[pc.type_of().index()] & self.color_bb[pc.color_of()]
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
        self.history[self.ply].moov()
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
        self.history[self.ply].half_move_counter() >= 100
    }

    fn is_repetition(&self) -> bool {
        let lookback = self.history[self.ply]
            .plies_from_null()
            .min(self.history[self.ply].half_move_counter()) as usize;

        self.history[self.ply - lookback..self.ply]
            .iter()
            .rev()
            .skip(1)
            .step_by(2)
            .any(|entry| self.material_hash() == entry.material_hash())
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

        self.history[self.ply] = HistoryEntry::default()
            .with_entry(self.history[self.ply - 1].entry())
            .with_half_move_counter(self.history[self.ply - 1].half_move_counter() + 1)
            .with_plies_from_null(0)
            .with_material_hash(self.history[self.ply - 1].material_hash());

        if let Some(epsq) = self.history[self.ply - 1].epsq() {
            self.hasher.update_ep(epsq.file());
        }

        self.hasher.update_color();
        self.ctm = !self.ctm;
    }

    pub fn pop_null(&mut self) {
        self.ply -= 1;
        self.hasher.update_color();

        if let Some(epsq) = self.history[self.ply].epsq() {
            self.hasher.update_ep(epsq.file());
        }
        self.ctm = !self.ctm;
    }

    pub fn push(&mut self, m: Move) {
        let mut half_move_counter = self.history[self.ply].half_move_counter() + 1;
        let mut captured = None;
        let mut epsq = None;
        self.ply += 1;

        if self.piece_type_at(m.from_sq()) == Some(PieceType::Pawn) {
            half_move_counter = 0;
        }

        match m.flags() {
            MoveFlags::Quiet => {
                self.move_piece_quiet(m.from_sq(), m.to_sq());
            }
            MoveFlags::DoublePush => {
                self.move_piece_quiet(m.from_sq(), m.to_sq());
                epsq = Some(m.from_sq() + Direction::North.relative(self.ctm));
                if let Some(sq) = epsq {
                    self.hasher.update_ep(sq.file());
                }
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
                self.move_piece_quiet(m.from_sq(), m.to_sq());
                self.remove_piece(m.to_sq() + Direction::South.relative(self.ctm));
            }
            MoveFlags::Capture => {
                captured = self.piece_at(m.to_sq());
                half_move_counter = 0;
                self.move_piece(m.from_sq(), m.to_sq());
            }
            // Promotions:
            _ => {
                if m.is_capture() {
                    captured = self.piece_at(m.to_sq());
                    self.remove_piece(m.to_sq());
                }
                self.remove_piece(m.from_sq());
                self.set_piece_at(
                    Piece::make_piece(
                        self.ctm,
                        m.promotion()
                            .expect("Tried to set a promotion piece for a non-promotion move."),
                    ),
                    m.to_sq(),
                );
            }
        };
        self.history[self.ply] = HistoryEntry::default()
            .with_entry(self.history[self.ply - 1].entry() | m.to_sq().bb() | m.from_sq().bb())
            .with_moov(Some(m))
            .with_half_move_counter(half_move_counter)
            .with_plies_from_null(self.history[self.ply - 1].plies_from_null() + 1)
            .with_captured(captured)
            .with_epsq(epsq)
            .with_material_hash(self.material_hash());
        self.ctm = !self.ctm;
        self.hasher.update_color();
    }

    pub fn pop(&mut self) -> Option<Move> {
        self.ctm = !self.ctm;
        self.hasher.update_color();

        let m = self.history[self.ply].moov()?;
        match m.flags() {
            MoveFlags::Quiet => {
                self.move_piece_quiet(m.to_sq(), m.from_sq());
            }
            MoveFlags::DoublePush => {
                self.move_piece_quiet(m.to_sq(), m.from_sq());
                if let Some(sq) = self.history[self.ply].epsq() {
                    self.hasher.update_ep(sq.file());
                }
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
                self.move_piece_quiet(m.to_sq(), m.from_sq());
                self.set_piece_at(
                    Piece::make_piece(!self.ctm, PieceType::Pawn),
                    m.to_sq() + Direction::South.relative(self.ctm),
                );
            }
            MoveFlags::PrKnight | MoveFlags::PrBishop | MoveFlags::PrRook | MoveFlags::PrQueen => {
                self.remove_piece(m.to_sq());
                self.set_piece_at(Piece::make_piece(self.ctm, PieceType::Pawn), m.from_sq());
            }
            MoveFlags::PcKnight | MoveFlags::PcBishop | MoveFlags::PcRook | MoveFlags::PcQueen => {
                self.remove_piece(m.to_sq());
                self.set_piece_at(Piece::make_piece(self.ctm, PieceType::Pawn), m.from_sq());
                self.set_piece_at(
                    self.history[self.ply]
                        .captured()
                        .expect("Tried to revert a capture move with no capture."),
                    m.to_sq(),
                );
            }
            MoveFlags::Capture => {
                self.move_piece_quiet(m.to_sq(), m.from_sq());
                self.set_piece_at(
                    self.history[self.ply]
                        .captured()
                        .expect("Tried to revert a capture move with no capture."),
                    m.to_sq(),
                );
            }
        }
        self.ply -= 1;
        Some(m)
    }

    pub fn generate_legal_moves<const QUIET: bool>(&self, moves: &mut MoveList) {
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
            .reduce(|a, b| a | b)
            .unwrap_or(Bitboard::ZERO);

        danger |= their_diag_sliders
            .map(|sq| attacks::bishop_attacks(sq, all ^ our_king.bb()))
            .reduce(|a, b| a | b)
            .unwrap_or(Bitboard::ZERO);

        danger |= their_orth_sliders
            .map(|sq| attacks::rook_attacks(sq, all ^ our_king.bb()))
            .reduce(|a, b| a | b)
            .unwrap_or(Bitboard::ZERO);

        ///////////////////////////////////////////////////////////////////
        // The king can move to any square that isn't attacked or occupied
        // by one of our pieces.
        ///////////////////////////////////////////////////////////////////

        let king_attacks = attacks::king_attacks(our_king) & !(us_bb | danger);

        if QUIET {
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
                            && self.history[self.ply].epsq().is_some_and(|epsq| {
                                checkers == epsq.bb().shift(Direction::South.relative(us))
                            })
                        {
                            let epsq = self.history[self.ply]
                                .epsq()
                                .expect("No epsq found for checker.");
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
                                moves.push(Move::new(sq, checker_square, MoveFlags::PcQueen));
                                if QUIET {
                                    moves.push(Move::new(sq, checker_square, MoveFlags::PcRook));
                                    moves.push(Move::new(sq, checker_square, MoveFlags::PcKnight));
                                    moves.push(Move::new(sq, checker_square, MoveFlags::PcBishop));
                                }
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
                if let Some(epsq) = self.history[self.ply].epsq() {
                    let epsq_attackers = attacks::pawn_attacks_sq(epsq, them)
                        & self.bitboard_of(us, PieceType::Pawn);
                    let unpinned_epsq_attackers = epsq_attackers & not_pinned;
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

                        if (attacks & their_orth_sliders) == Bitboard::ZERO {
                            moves.push(Move::new(sq, epsq, MoveFlags::EnPassant));
                        }
                    }
                    ///////////////////////////////////////////////////////////////////
                    // Pinned pawns can only capture ep if they are pinned diagonally
                    // and the ep square is in line with the king.
                    ///////////////////////////////////////////////////////////////////
                    let pinned_epsq_attackers =
                        epsq_attackers & pinned & Bitboard::line(epsq, our_king);
                    if pinned_epsq_attackers != Bitboard::ZERO {
                        moves.push(Move::new(
                            pinned_epsq_attackers.lsb(),
                            epsq,
                            MoveFlags::EnPassant,
                        ));
                    }
                }

                ///////////////////////////////////////////////////////////////////
                // Only castle if:
                // 1. Neither the king nor rook have moved.
                // 2. The king is not in check.
                // 3. The relevant squares are not attacked.
                ///////////////////////////////////////////////////////////////////
                if QUIET {
                    if ((self.history[self.ply].entry() & Bitboard::oo_mask(us))
                        | ((all | danger) & Bitboard::oo_blockers_mask(us)))
                        == Bitboard::ZERO
                    {
                        moves.push(match us {
                            Color::White => Move::new(SQ::E1, SQ::G1, MoveFlags::OO),
                            Color::Black => Move::new(SQ::E8, SQ::G8, MoveFlags::OO),
                        });
                    }
                    if ((self.history[self.ply].entry() & Bitboard::ooo_mask(us))
                        | ((all | (danger & !Bitboard::ignore_ooo_danger(us)))
                            & Bitboard::ooo_blockers_mask(us)))
                        == Bitboard::ZERO
                    {
                        moves.push(match us {
                            Color::White => Move::new(SQ::E1, SQ::C1, MoveFlags::OOO),
                            Color::Black => Move::new(SQ::E8, SQ::C8, MoveFlags::OOO),
                        });
                    }
                }
                ///////////////////////////////////////////////////////////////////
                // For each pinned rook, bishop, or queen, only include attacks
                // that are aligned with our king.
                ///////////////////////////////////////////////////////////////////
                let pinned_pieces = !(not_pinned | self.bitboard_of(us, PieceType::Knight));
                for sq in pinned_pieces {
                    let pt = self
                        .piece_type_at(sq)
                        .expect("Unexpected None for piece type.");
                    let attacks_along_pin =
                        attacks::attacks(pt, sq, all) & Bitboard::line(our_king, sq);
                    if QUIET {
                        moves.make_q(sq, attacks_along_pin & quiet_mask);
                    }
                    moves.make_c(sq, attacks_along_pin & capture_mask);
                }

                ///////////////////////////////////////////////////////////////////
                // For each pinned pawn
                ///////////////////////////////////////////////////////////////////
                let pinned_pawns = !not_pinned & self.bitboard_of(us, PieceType::Pawn);
                for sq in pinned_pawns {
                    ///////////////////////////////////////////////////////////////////
                    // Quiet promotions are impossible since the square in front of the
                    // pawn will be occupied
                    ///////////////////////////////////////////////////////////////////
                    if sq.rank() == Rank::Seven.relative(us) {
                        moves.make_pc(
                            sq,
                            attacks::pawn_attacks_sq(sq, us)
                                & capture_mask
                                & Bitboard::line(our_king, sq),
                        );
                    } else {
                        moves.make_c(
                            sq,
                            attacks::pawn_attacks_sq(sq, us)
                                & them_bb
                                & Bitboard::line(sq, our_king),
                        );

                        ///////////////////////////////////////////////////////////////////
                        // Single and double pawn pushes
                        ///////////////////////////////////////////////////////////////////
                        if QUIET {
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
        }

        ///////////////////////////////////////////////////////////////////
        // Non-pinned moves from here
        ///////////////////////////////////////////////////////////////////
        for sq in self.bitboard_of(us, PieceType::Knight) & not_pinned {
            let knight_attacks = attacks::knight_attacks(sq);
            moves.make_c(sq, knight_attacks & capture_mask);
            if QUIET {
                moves.make_q(sq, knight_attacks & quiet_mask);
            }
        }

        for sq in our_diag_sliders & not_pinned {
            let diag_attacks = attacks::bishop_attacks(sq, all);
            moves.make_c(sq, diag_attacks & capture_mask);
            if QUIET {
                moves.make_q(sq, diag_attacks & quiet_mask);
            }
        }

        for sq in our_orth_sliders & not_pinned {
            let orth_attacks = attacks::rook_attacks(sq, all);
            moves.make_c(sq, orth_attacks & capture_mask);
            if QUIET {
                moves.make_q(sq, orth_attacks & quiet_mask);
            }
        }

        let back_pawns =
            self.bitboard_of(us, PieceType::Pawn) & not_pinned & !Rank::Seven.relative(us).bb();
        let mut single_pushes = back_pawns.shift(Direction::North.relative(us)) & !all;
        let double_pushes = (single_pushes & Rank::Three.relative(us).bb())
            .shift(Direction::North.relative(us))
            & quiet_mask;

        single_pushes &= quiet_mask;

        if QUIET {
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

        let northwest_captures = back_pawns.shift(Direction::NorthWest.relative(us)) & capture_mask;
        let northeast_captures = back_pawns.shift(Direction::NorthEast.relative(us)) & capture_mask;

        for sq in northwest_captures {
            moves.push(Move::new(
                sq - Direction::NorthWest.relative(us),
                sq,
                MoveFlags::Capture,
            ));
        }

        for sq in northeast_captures {
            moves.push(Move::new(
                sq - Direction::NorthEast.relative(us),
                sq,
                MoveFlags::Capture,
            ));
        }

        let seventh_rank_pawns =
            self.bitboard_of(us, PieceType::Pawn) & not_pinned & Rank::Seven.relative(us).bb();

        if seventh_rank_pawns != Bitboard::ZERO {
            let quiet_promotions =
                seventh_rank_pawns.shift(Direction::North.relative(us)) & quiet_mask;
            for sq in quiet_promotions {
                moves.push(Move::new(
                    sq - Direction::North.relative(us),
                    sq,
                    MoveFlags::PrQueen,
                ));
                if QUIET {
                    moves.push(Move::new(
                        sq - Direction::North.relative(us),
                        sq,
                        MoveFlags::PrRook,
                    ));
                    moves.push(Move::new(
                        sq - Direction::North.relative(us),
                        sq,
                        MoveFlags::PrKnight,
                    ));
                    moves.push(Move::new(
                        sq - Direction::North.relative(us),
                        sq,
                        MoveFlags::PrBishop,
                    ));
                }
            }

            let northwest_promotions =
                seventh_rank_pawns.shift(Direction::NorthWest.relative(us)) & capture_mask;
            let northeast_promotions =
                seventh_rank_pawns.shift(Direction::NorthEast.relative(us)) & capture_mask;
            for sq in northwest_promotions {
                moves.push(Move::new(
                    sq - Direction::NorthWest.relative(us),
                    sq,
                    MoveFlags::PcQueen,
                ));
                if QUIET {
                    moves.push(Move::new(
                        sq - Direction::NorthWest.relative(us),
                        sq,
                        MoveFlags::PcRook,
                    ));
                    moves.push(Move::new(
                        sq - Direction::NorthWest.relative(us),
                        sq,
                        MoveFlags::PcKnight,
                    ));
                    moves.push(Move::new(
                        sq - Direction::NorthWest.relative(us),
                        sq,
                        MoveFlags::PcBishop,
                    ));
                }
            }

            for sq in northeast_promotions {
                moves.push(Move::new(
                    sq - Direction::NorthEast.relative(us),
                    sq,
                    MoveFlags::PcQueen,
                ));
                if QUIET {
                    moves.push(Move::new(
                        sq - Direction::NorthEast.relative(us),
                        sq,
                        MoveFlags::PcRook,
                    ));
                    moves.push(Move::new(
                        sq - Direction::NorthEast.relative(us),
                        sq,
                        MoveFlags::PcKnight,
                    ));
                    moves.push(Move::new(
                        sq - Direction::NorthEast.relative(us),
                        sq,
                        MoveFlags::PcBishop,
                    ));
                }
            }
        }
    }

    pub fn push_str(&mut self, move_str: &str) -> Result<(), &'static str> {
        let m = MoveList::from(self)
            .iter_moves()
            .find(|m| m.to_string() == move_str)
            .ok_or("Invalid move.")?;

        self.push(m);
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

        if piece_placement.split('/').count() != Rank::N_RANKS {
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
            self.hasher.update_color();
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
                    let sq = SQ::from(idx as u8);
                    let pc = Piece::try_from(ch)?;
                    self.set_piece_at(pc, sq);
                    idx += 1;
                }
            }

            if idx != 64 - 8 * rank_idx {
                return Err("FEN rank does not fill expected number of squares.");
            }
        }

        self.history[self.ply] = self.history[self.ply].with_entry(Bitboard::ALL_CASTLING_MASK);
        for (symbol, mask) in [
            ('K', Bitboard::WHITE_OO_MASK),
            ('Q', Bitboard::WHITE_OOO_MASK),
            ('k', Bitboard::BLACK_OO_MASK),
            ('q', Bitboard::BLACK_OOO_MASK),
        ] {
            if castling.contains(symbol) {
                self.history[self.ply] =
                    self.history[self.ply].with_entry(self.history[self.ply].entry() & !mask);
            }
        }

        if en_passant_sq != "-" {
            let epsq = SQ::try_from(en_passant_sq)?;
            self.history[self.ply] = self.history[self.ply].with_epsq(Some(epsq));
            self.hasher.update_ep(epsq.file());
        }
        self.history[self.ply] = self.history[self.ply].with_half_move_counter(
            halfmove_clock
                .parse::<u16>()
                .map_err(|_| "Invalid half move counter.")?,
        );
        self.history[self.ply] =
            self.history[self.ply].with_material_hash(self.hasher.material_hash());
        Ok(())
    }

    pub fn ctm(&self) -> Color {
        self.ctm
    }

    pub fn ply(&self) -> usize {
        self.ply
    }

    pub fn hash(&self) -> Hash {
        self.hasher.hash()
    }

    pub fn material_hash(&self) -> Hash {
        self.hasher.material_hash()
    }

    pub fn fullmove_number(&self) -> usize {
        self.ply / 2 + 1
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            piece_type_bb: PieceTypeMap::new([Bitboard::ZERO; PieceType::N_PIECE_TYPES]),
            color_bb: ColorMap::new([Bitboard::ZERO; Color::N_COLORS]),
            board: SQMap::new([None; SQ::N_SQUARES]),
            ctm: Color::White,
            ply: 0,
            hasher: Hasher::new(),
            network: Network::new(),
            history: [HistoryEntry::default(); Self::N_HISTORIES],
        }
    }
}

impl TryFrom<&str> for Board {
    type Error = &'static str;

    fn try_from(fen: &str) -> Result<Self, Self::Error> {
        let mut board = Board::default();
        board.set_fen(fen)?;
        Ok(board)
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut board_str = String::new();
        for rank_idx in (0..=7).rev() {
            let rank = Rank::from(rank_idx);
            let mut empty_squares = 0;
            for file_idx in 0..=7 {
                let file = File::from(file_idx);
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

        let mut castling_rights_str = String::new();
        for (symbol, mask) in "KQkq".chars().zip([
            Bitboard::WHITE_OO_MASK,
            Bitboard::WHITE_OOO_MASK,
            Bitboard::BLACK_OO_MASK,
            Bitboard::BLACK_OOO_MASK,
        ]) {
            if mask & self.history[self.ply].entry() == Bitboard::ZERO {
                castling_rights_str.push(symbol);
            }
        }
        if castling_rights_str.is_empty() {
            castling_rights_str = "-".to_string();
        }

        let epsq_str = match self.history[self.ply].epsq() {
            Some(epsq) => epsq.to_string(),
            None => "-".to_string(),
        };

        write!(
            f,
            "{} {} {} {} {} {}",
            board_str,
            self.ctm,
            castling_rights_str,
            epsq_str,
            self.history[self.ply].half_move_counter(),
            self.ply / 2 + 1,
        )
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::with_capacity(SQ::N_SQUARES * 2 + 8);
        for rank_idx in (0..=7).rev() {
            let rank = Rank::from(rank_idx);
            for file_idx in 0..=7 {
                let file = File::from(file_idx);
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
        write!(f, "{}", s)
    }
}

impl Board {
    const N_HISTORIES: usize = 1000;
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
    entry: Bitboard,
    captured: Option<Piece>,
    epsq: Option<SQ>,
    moov: Option<Move>,
    material_hash: Hash,
    half_move_counter: u16,
    plies_from_null: u16,
}

impl HistoryEntry {
    pub fn entry(&self) -> Bitboard {
        self.entry
    }

    pub fn moov(&self) -> Option<Move> {
        self.moov
    }

    pub fn captured(&self) -> Option<Piece> {
        self.captured
    }

    pub fn epsq(&self) -> Option<SQ> {
        self.epsq
    }

    pub fn half_move_counter(&self) -> u16 {
        self.half_move_counter
    }

    pub fn plies_from_null(&self) -> u16 {
        self.plies_from_null
    }

    pub fn material_hash(&self) -> Hash {
        self.material_hash
    }

    pub fn with_entry(&mut self, entry: Bitboard) -> Self {
        self.entry = entry;
        *self
    }

    pub fn with_moov(&mut self, moov: Option<Move>) -> Self {
        self.moov = moov;
        *self
    }

    pub fn with_captured(&mut self, pc: Option<Piece>) -> Self {
        self.captured = pc;
        *self
    }

    pub fn with_epsq(&mut self, sq: Option<SQ>) -> Self {
        self.epsq = sq;
        *self
    }

    pub fn with_half_move_counter(&mut self, half_move_counter: u16) -> Self {
        self.half_move_counter = half_move_counter;
        *self
    }

    pub fn with_plies_from_null(&mut self, plies_from_null: u16) -> Self {
        self.plies_from_null = plies_from_null;
        *self
    }

    pub fn with_material_hash(&mut self, material_hash: Hash) -> Self {
        self.material_hash = material_hash;
        *self
    }
}

#[cfg(test)]
mod tests {
    use crate::board::*;

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
