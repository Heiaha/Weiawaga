use super::attacks;
use super::bitboard::*;
use super::color::*;
use super::file::*;
use super::moov::*;
use super::move_list::*;
use super::piece::*;
use super::rank::*;
use super::square::*;
use super::undo_info::*;
use super::zobrist;
use crate::evaluation::e_constants;
use crate::evaluation::score::*;
use std::cmp::min;
use std::convert::TryFrom;
use std::fmt;

#[derive(Copy, Clone)]
pub struct Board {
    piece_bb: [BitBoard; N_PIECES],
    board: [Piece; N_SQUARES],
    color_bb: [BitBoard; N_COLORS],
    color_to_play: Color,
    hash: Hash,
    material_hash: Hash,
    game_ply: usize,
    phase: Phase,
    material_score: Score,
    p_sq_score: Score,
    history: [UndoInfo; 1000],
    checkers: BitBoard,
    pinned: BitBoard,
}

impl Board {
    pub fn new() -> Self {
        let mut board = Self::clean();
        board
            .set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        board
    }

    pub fn clean() -> Self {
        Board {
            piece_bb: [BitBoard::ZERO; N_PIECES],
            color_bb: [BitBoard::ZERO; N_COLORS],
            board: [Piece::None; N_SQUARES],
            color_to_play: Color::White,
            hash: BitBoard::ZERO,
            material_hash: BitBoard::ZERO,
            game_ply: 0,
            phase: Score::TOTAL_PHASE,
            material_score: Score::ZERO,
            p_sq_score: Score::ZERO,
            history: [UndoInfo::default(); 1000],
            checkers: BitBoard::ZERO,
            pinned: BitBoard::ZERO,
        }
    }

    pub fn clear(&mut self) {
        self.color_to_play = Color::White;
        self.hash = BitBoard::ZERO;
        self.material_hash = BitBoard::ZERO;
        self.phase = Score::TOTAL_PHASE;
        self.material_score = Score::ZERO;
        self.p_sq_score = Score::ZERO;
        self.history = [UndoInfo::default(); 1000];
        self.checkers = BitBoard::ZERO;
        self.pinned = BitBoard::ZERO;

        self.color_bb[Color::White.index()] = BitBoard::ZERO;
        self.color_bb[Color::Black.index()] = BitBoard::ZERO;

        for pc in Piece::iter(Piece::WhitePawn, Piece::BlackKing) {
            self.piece_bb[pc.index()] = BitBoard::ZERO;
        }

        for sq in BitBoard::ALL {
            self.board[sq.index()] = Piece::None;
        }
    }

    #[inline(always)]
    pub fn piece_at(&self, sq: SQ) -> Piece {
        self.board[sq.index()]
    }

    #[inline(always)]
    pub fn piece_type_at(&self, sq: SQ) -> PieceType {
        self.board[sq.index()].type_of()
    }

    pub fn set_piece_at(&mut self, pc: Piece, sq: SQ) {
        self.phase -= Score::piece_phase(pc.type_of());
        self.p_sq_score += e_constants::piece_sq_value(pc, sq);
        self.material_score += e_constants::piece_score(pc);

        self.board[sq.index()] = pc;
        self.color_bb[pc.color_of().index()] |= sq.bb();
        self.piece_bb[pc.index()] |= sq.bb();

        self.hash ^= zobrist::zobrist_table(pc, sq);
        self.material_hash ^= zobrist::zobrist_table(pc, sq);
    }

    pub fn remove_piece(&mut self, sq: SQ) {
        let pc = self.piece_at(sq);
        self.phase += Score::piece_phase(pc.type_of());
        self.p_sq_score -= e_constants::piece_sq_value(pc, sq);
        self.material_score -= e_constants::piece_score(pc);

        self.hash ^= zobrist::zobrist_table(pc, sq);
        self.material_hash ^= zobrist::zobrist_table(pc, sq);

        self.piece_bb[pc.index()] &= !sq.bb();
        self.color_bb[pc.color_of().index()] &= !sq.bb();
        self.board[sq.index()] = Piece::None;
    }

    pub fn move_piece_quiet(&mut self, from_sq: SQ, to_sq: SQ) {
        let pc = self.piece_at(from_sq);
        self.p_sq_score +=
            e_constants::piece_sq_value(pc, to_sq) - e_constants::piece_sq_value(pc, from_sq);

        let hash_update = zobrist::zobrist_table(pc, from_sq) ^ zobrist::zobrist_table(pc, to_sq);
        self.hash ^= hash_update;
        self.material_hash ^= hash_update;

        let mask = from_sq.bb() | to_sq.bb();
        self.piece_bb[pc.index()] ^= mask;
        self.color_bb[pc.color_of().index()] ^= mask;
        self.board[to_sq.index()] = self.board[from_sq.index()];
        self.board[from_sq.index()] = Piece::None;
    }

    pub fn move_piece(&mut self, from_sq: SQ, to_sq: SQ) {
        self.remove_piece(to_sq);
        self.move_piece_quiet(from_sq, to_sq);
    }

    #[inline(always)]
    pub fn bitboard_of_piece(&self, pc: Piece) -> BitBoard {
        self.piece_bb[pc.index()]
    }

    #[inline(always)]
    pub fn bitboard_of(&self, c: Color, pt: PieceType) -> BitBoard {
        self.piece_bb[Piece::make_piece(c, pt).index()]
    }

    #[inline(always)]
    pub fn bitboard_of_piecetype(&self, pt: PieceType) -> BitBoard {
        self.piece_bb[Piece::make_piece(Color::White, pt).index()]
            | self.piece_bb[Piece::make_piece(Color::Black, pt).index()]
    }

    pub fn diagonal_sliders(&self, color: Color) -> BitBoard {
        match color {
            Color::White => {
                self.piece_bb[Piece::WhiteBishop.index()] | self.piece_bb[Piece::WhiteQueen.index()]
            }
            Color::Black => {
                self.piece_bb[Piece::BlackBishop.index()] | self.piece_bb[Piece::BlackQueen.index()]
            }
        }
    }

    pub fn orthogonal_sliders(&self, color: Color) -> BitBoard {
        match color {
            Color::White => {
                self.piece_bb[Piece::WhiteRook.index()] | self.piece_bb[Piece::WhiteQueen.index()]
            }
            Color::Black => {
                self.piece_bb[Piece::BlackRook.index()] | self.piece_bb[Piece::BlackQueen.index()]
            }
        }
    }

    #[inline(always)]
    pub fn all_pieces(&self) -> BitBoard {
        self.color_bb[Color::White.index()] | self.color_bb[Color::Black.index()]
    }

    #[inline(always)]
    pub fn all_pieces_color(&self, color: Color) -> BitBoard {
        self.color_bb[color.index()]
    }

    pub fn attackers_from_color(&self, sq: SQ, occ: BitBoard, color: Color) -> BitBoard {
        match color {
            Color::White => {
                (self.piece_bb[Piece::WhitePawn.index()]
                    & attacks::pawn_attacks_sq(sq, Color::Black))
                    | (self.piece_bb[Piece::WhiteKnight.index()] & attacks::knight_attacks(sq))
                    | (self.piece_bb[Piece::WhiteBishop.index()] & attacks::bishop_attacks(sq, occ))
                    | (self.piece_bb[Piece::WhiteRook.index()] & attacks::rook_attacks(sq, occ))
                    | (self.piece_bb[Piece::WhiteQueen.index()]
                        & (attacks::bishop_attacks(sq, occ) | attacks::rook_attacks(sq, occ)))
            }
            Color::Black => {
                (self.piece_bb[Piece::BlackPawn.index()]
                    & attacks::pawn_attacks_sq(sq, Color::White))
                    | (self.piece_bb[Piece::BlackKnight.index()] & attacks::knight_attacks(sq))
                    | (self.piece_bb[Piece::BlackBishop.index()] & attacks::bishop_attacks(sq, occ))
                    | (self.piece_bb[Piece::BlackRook.index()] & attacks::rook_attacks(sq, occ))
                    | (self.piece_bb[Piece::BlackQueen.index()]
                        & (attacks::bishop_attacks(sq, occ) | attacks::rook_attacks(sq, occ)))
            }
        }
    }

    pub fn attackers_to_square(&self, sq: SQ, occ: BitBoard) -> BitBoard {
        (self.piece_bb[Piece::WhitePawn.index()] & attacks::pawn_attacks_sq(sq, Color::Black))
            | (self.piece_bb[Piece::WhiteKnight.index()] & attacks::knight_attacks(sq))
            | (self.piece_bb[Piece::WhiteBishop.index()] & attacks::bishop_attacks(sq, occ))
            | (self.piece_bb[Piece::WhiteRook.index()] & attacks::rook_attacks(sq, occ))
            | (self.piece_bb[Piece::WhiteQueen.index()]
                & (attacks::bishop_attacks(sq, occ) | attacks::rook_attacks(sq, occ)))
            | (self.piece_bb[Piece::BlackPawn.index()] & attacks::pawn_attacks_sq(sq, Color::White))
            | (self.piece_bb[Piece::BlackKnight.index()] & attacks::knight_attacks(sq))
            | (self.piece_bb[Piece::BlackBishop.index()] & attacks::bishop_attacks(sq, occ))
            | (self.piece_bb[Piece::BlackRook.index()] & attacks::rook_attacks(sq, occ))
            | (self.piece_bb[Piece::BlackQueen.index()]
                & (attacks::bishop_attacks(sq, occ) | attacks::rook_attacks(sq, occ)))
    }

    pub fn in_check(&self) -> bool {
        let us = self.color_to_play;
        let them = !self.color_to_play;
        let our_king = self.bitboard_of(us, PieceType::King).lsb();

        if attacks::knight_attacks(our_king) & self.bitboard_of(them, PieceType::Knight)
            != BitBoard::ZERO
        {
            return true;
        }

        if attacks::pawn_attacks_sq(our_king, us) & self.bitboard_of(them, PieceType::Pawn)
            != BitBoard::ZERO
        {
            return true;
        }

        let all = self.all_pieces_color(us) | self.all_pieces_color(them);
        if attacks::rook_attacks(our_king, all) & self.orthogonal_sliders(them) != BitBoard::ZERO {
            return true;
        }

        if attacks::bishop_attacks(our_king, all) & self.diagonal_sliders(them) != BitBoard::ZERO {
            return true;
        }
        false
    }

    #[inline(always)]
    pub fn peek_capture(&self) -> PieceType {
        self.history[self.game_ply].captured().type_of()
    }

    #[inline(always)]
    pub fn peek(&self) -> Move {
        self.history[self.game_ply].moove()
    }

    #[inline(always)]
    fn is_insufficient_material(&self) -> bool {
        (self.bitboard_of_piecetype(PieceType::Pawn)
            | self.bitboard_of_piecetype(PieceType::Rook)
            | self.bitboard_of_piecetype(PieceType::Queen))
            == BitBoard::ZERO
            && (!self.all_pieces_color(Color::White).is_several()
                || !self.all_pieces_color(Color::Black).is_several())
            && (!(self.bitboard_of_piecetype(PieceType::Knight)
                | self.bitboard_of_piecetype(PieceType::Bishop))
            .is_several()
                || (self.bitboard_of_piecetype(PieceType::Bishop) == BitBoard::ZERO
                    && self.bitboard_of_piecetype(PieceType::Knight).pop_count() <= 2))
    }

    #[inline(always)]
    fn is_fifty(&self) -> bool {
        self.history[self.game_ply].half_move_counter() >= 100
    }

    fn is_threefold(&self) -> bool {
        let lookback = min(
            self.history[self.game_ply].plies_from_null(),
            self.history[self.game_ply].half_move_counter(),
        ) as usize;
        for i in (2..=lookback).step_by(2) {
            if self.material_hash == self.history[self.game_ply - i].material_hash() {
                return true;
            }
        }
        false
    }

    #[inline(always)]
    pub fn is_draw(&self) -> bool {
        self.is_fifty() || self.is_insufficient_material() || self.is_threefold()
    }

    pub fn has_non_pawn_material(&self) -> bool {
        for pt in PieceType::iter(PieceType::Knight, PieceType::Queen) {
            if self.bitboard_of(self.color_to_play, pt) != BitBoard::ZERO {
                return true;
            }
        }
        false
    }

    pub fn push_null(&mut self) {
        self.game_ply += 1;
        self.history[self.game_ply] = UndoInfo::new(
            self.history[self.game_ply - 1].entry(),
            Move::NULL,
            self.history[self.game_ply - 1].half_move_counter() + 1,
            0,
            Piece::None,
            SQ::None,
            self.history[self.game_ply - 1].material_hash(),
        );

        if self.history[self.game_ply - 1].epsq() != SQ::None {
            self.hash ^= zobrist::zobrist_ep(self.history[self.game_ply - 1].epsq().file());
        }

        self.hash ^= zobrist::zobrist_color();
        self.color_to_play = !self.color_to_play;
    }

    pub fn pop_null(&mut self) {
        self.game_ply -= 1;
        self.hash ^= zobrist::zobrist_color();
        if self.history[self.game_ply].epsq() != SQ::None {
            self.hash ^= zobrist::zobrist_ep(self.history[self.game_ply].epsq().file());
        }
        self.color_to_play = !self.color_to_play;
    }

    pub fn push(&mut self, m: Move) {
        let mut half_move_counter = self.history[self.game_ply].half_move_counter() + 1;
        let mut captured = Piece::None;
        let mut epsq = SQ::None;
        self.game_ply += 1;

        if self.piece_at(m.from_sq()).type_of() == PieceType::Pawn {
            half_move_counter = 0;
        }

        match m.flags() {
            MoveFlags::Quiet => {
                self.move_piece_quiet(m.from_sq(), m.to_sq());
            }
            MoveFlags::DoublePush => {
                self.move_piece_quiet(m.from_sq(), m.to_sq());
                epsq = m.from_sq() + Direction::North.relative(self.color_to_play);
                self.hash ^= zobrist::zobrist_ep(epsq.file());
            }
            MoveFlags::OO => {
                if self.color_to_play == Color::White {
                    self.move_piece_quiet(SQ::E1, SQ::G1);
                    self.move_piece_quiet(SQ::H1, SQ::F1);
                } else {
                    self.move_piece_quiet(SQ::E8, SQ::G8);
                    self.move_piece_quiet(SQ::H8, SQ::F8);
                }
            }
            MoveFlags::OOO => {
                if self.color_to_play == Color::White {
                    self.move_piece_quiet(SQ::E1, SQ::C1);
                    self.move_piece_quiet(SQ::A1, SQ::D1);
                } else {
                    self.move_piece_quiet(SQ::E8, SQ::C8);
                    self.move_piece_quiet(SQ::A8, SQ::D8);
                }
            }
            MoveFlags::EnPassant => {
                self.move_piece_quiet(m.from_sq(), m.to_sq());
                self.remove_piece(m.to_sq() + Direction::South.relative(self.color_to_play));
            }
            MoveFlags::PrKnight => {
                self.remove_piece(m.from_sq());
                self.set_piece_at(
                    Piece::make_piece(self.color_to_play, PieceType::Knight),
                    m.to_sq(),
                );
            }
            MoveFlags::PrBishop => {
                self.remove_piece(m.from_sq());
                self.set_piece_at(
                    Piece::make_piece(self.color_to_play, PieceType::Bishop),
                    m.to_sq(),
                );
            }
            MoveFlags::PrRook => {
                self.remove_piece(m.from_sq());
                self.set_piece_at(
                    Piece::make_piece(self.color_to_play, PieceType::Rook),
                    m.to_sq(),
                );
            }
            MoveFlags::PrQueen => {
                self.remove_piece(m.from_sq());
                self.set_piece_at(
                    Piece::make_piece(self.color_to_play, PieceType::Queen),
                    m.to_sq(),
                );
            }
            MoveFlags::PcKnight => {
                captured = self.piece_at(m.to_sq());
                self.remove_piece(m.from_sq());
                self.remove_piece(m.to_sq());
                self.set_piece_at(
                    Piece::make_piece(self.color_to_play, PieceType::Knight),
                    m.to_sq(),
                );
            }
            MoveFlags::PcBishop => {
                captured = self.piece_at(m.to_sq());
                self.remove_piece(m.from_sq());
                self.remove_piece(m.to_sq());
                self.set_piece_at(
                    Piece::make_piece(self.color_to_play, PieceType::Bishop),
                    m.to_sq(),
                );
            }
            MoveFlags::PcRook => {
                captured = self.piece_at(m.to_sq());
                self.remove_piece(m.from_sq());
                self.remove_piece(m.to_sq());
                self.set_piece_at(
                    Piece::make_piece(self.color_to_play, PieceType::Rook),
                    m.to_sq(),
                );
            }
            MoveFlags::PcQueen => {
                captured = self.piece_at(m.to_sq());
                self.remove_piece(m.from_sq());
                self.remove_piece(m.to_sq());
                self.set_piece_at(
                    Piece::make_piece(self.color_to_play, PieceType::Queen),
                    m.to_sq(),
                );
            }
            MoveFlags::Capture => {
                captured = self.piece_at(m.to_sq());
                half_move_counter = 0;
                self.move_piece(m.from_sq(), m.to_sq());
            }
        };
        self.history[self.game_ply] = UndoInfo::new(
            self.history[self.game_ply - 1].entry() | m.to_sq().bb() | m.from_sq().bb(),
            m,
            half_move_counter,
            self.history[self.game_ply - 1].plies_from_null() + 1,
            captured,
            epsq,
            self.material_hash,
        );
        self.color_to_play = !self.color_to_play;
        self.hash ^= zobrist::zobrist_color();
    }

    pub fn pop(&mut self) -> Move {
        self.color_to_play = !self.color_to_play;
        self.hash ^= zobrist::zobrist_color();

        let m = self.history[self.game_ply].moove();
        match m.flags() {
            MoveFlags::Quiet => {
                self.move_piece_quiet(m.to_sq(), m.from_sq());
            }
            MoveFlags::DoublePush => {
                self.move_piece_quiet(m.to_sq(), m.from_sq());
                self.hash ^= zobrist::zobrist_ep(self.history[self.game_ply].epsq().file());
            }
            MoveFlags::OO => {
                if self.color_to_play == Color::White {
                    self.move_piece_quiet(SQ::G1, SQ::E1);
                    self.move_piece_quiet(SQ::F1, SQ::H1);
                } else {
                    self.move_piece_quiet(SQ::G8, SQ::E8);
                    self.move_piece_quiet(SQ::F8, SQ::H8);
                }
            }
            MoveFlags::OOO => {
                if self.color_to_play == Color::White {
                    self.move_piece_quiet(SQ::C1, SQ::E1);
                    self.move_piece_quiet(SQ::D1, SQ::A1);
                } else {
                    self.move_piece_quiet(SQ::C8, SQ::E8);
                    self.move_piece_quiet(SQ::D8, SQ::A8);
                }
            }
            MoveFlags::EnPassant => {
                self.move_piece_quiet(m.to_sq(), m.from_sq());
                self.set_piece_at(
                    Piece::make_piece(!self.color_to_play, PieceType::Pawn),
                    m.to_sq() + Direction::South.relative(self.color_to_play),
                );
            }
            MoveFlags::PrKnight | MoveFlags::PrBishop | MoveFlags::PrRook | MoveFlags::PrQueen => {
                self.remove_piece(m.to_sq());
                self.set_piece_at(
                    Piece::make_piece(self.color_to_play, PieceType::Pawn),
                    m.from_sq(),
                );
            }
            MoveFlags::PcKnight | MoveFlags::PcBishop | MoveFlags::PcRook | MoveFlags::PcQueen => {
                self.remove_piece(m.to_sq());
                self.set_piece_at(
                    Piece::make_piece(self.color_to_play, PieceType::Pawn),
                    m.from_sq(),
                );
                self.set_piece_at(self.history[self.game_ply].captured(), m.to_sq());
            }
            MoveFlags::Capture => {
                self.move_piece_quiet(m.to_sq(), m.from_sq());
                self.set_piece_at(self.history[self.game_ply].captured(), m.to_sq());
            }
        }
        self.game_ply -= 1;
        m
    }

    pub fn generate_legal_moves(&mut self, moves: &mut MoveList) {
        let us = self.color_to_play;
        let them = !self.color_to_play;

        let us_bb = self.all_pieces_color(us);
        let them_bb = self.all_pieces_color(them);
        let all = us_bb | them_bb;

        let our_king = self
            .bitboard_of_piece(Piece::make_piece(us, PieceType::King))
            .lsb();
        let their_king = self
            .bitboard_of_piece(Piece::make_piece(them, PieceType::King))
            .lsb();

        let our_diag_sliders = self.diagonal_sliders(us);
        let their_diag_sliders = self.diagonal_sliders(them);
        let our_orth_sliders = self.orthogonal_sliders(us);
        let their_orth_sliders = self.orthogonal_sliders(them);

        ///////////////////////////////////////////////////////////////////
        // General purpose bitboards.
        ///////////////////////////////////////////////////////////////////

        let mut b1: BitBoard;
        let mut b2: BitBoard;
        let mut b3: BitBoard;

        ///////////////////////////////////////////////////////////////////
        // Danger squares for the king
        ///////////////////////////////////////////////////////////////////
        let mut danger = BitBoard::ZERO;

        ///////////////////////////////////////////////////////////////////
        // Add each enemy attack to the danger bitboard
        ///////////////////////////////////////////////////////////////////
        danger |= attacks::pawn_attacks_bb(self.bitboard_of(them, PieceType::Pawn), them)
            | attacks::king_attacks(their_king);

        b1 = self.bitboard_of(them, PieceType::Knight);
        for sq in b1 {
            danger |= attacks::knight_attacks(sq);
        }

        b1 = their_diag_sliders;
        for sq in b1 {
            danger |= attacks::bishop_attacks(sq, all ^ our_king.bb());
        }

        b1 = their_orth_sliders;
        for sq in b1 {
            danger |= attacks::rook_attacks(sq, all ^ our_king.bb());
        }

        ///////////////////////////////////////////////////////////////////
        // The king can move to any square that isn't attacked or occupied
        // by one of our pieces.
        ///////////////////////////////////////////////////////////////////

        b1 = attacks::king_attacks(our_king) & !(us_bb | danger);

        moves.make_q(our_king, b1 & !them_bb);
        moves.make_c(our_king, b1 & them_bb);

        ///////////////////////////////////////////////////////////////////
        // The capture mask consists of destination squares containing enemy
        // pieces that must be captured because they are checking the king.
        ///////////////////////////////////////////////////////////////////
        let capture_mask: BitBoard;

        ///////////////////////////////////////////////////////////////////
        // The quiet mask consists of squares where pieces must be moved
        // to block an attack checking the king.
        ///////////////////////////////////////////////////////////////////
        let quiet_mask: BitBoard;

        ///////////////////////////////////////////////////////////////////
        // Checkers are identified by projecting attacks from the king
        // square and then intersecting them with the enemy bitboard of the
        // respective piece.
        ///////////////////////////////////////////////////////////////////
        self.checkers = (attacks::knight_attacks(our_king)
            & self.bitboard_of(them, PieceType::Knight))
            | (attacks::pawn_attacks_sq(our_king, us) & self.bitboard_of(them, PieceType::Pawn));

        ///////////////////////////////////////////////////////////////////
        // Candidates are potential slider checkers and pinners.
        ///////////////////////////////////////////////////////////////////
        let candidates = (attacks::rook_attacks(our_king, them_bb) & their_orth_sliders)
            | (attacks::bishop_attacks(our_king, them_bb) & their_diag_sliders);

        self.pinned = BitBoard::ZERO;
        for sq in candidates {
            b1 = BitBoard::between(our_king, sq) & us_bb;

            ///////////////////////////////////////////////////////////////////
            // Do the squares between an enemy slider and our king contain any
            // pieces? If yes, that piece is pinned. Otherwise, we are checked.
            ///////////////////////////////////////////////////////////////////
            if b1 == BitBoard::ZERO {
                self.checkers ^= sq.bb();
            } else if b1.is_single() {
                self.pinned ^= b1;
            }
        }

        let not_pinned = !self.pinned;

        match self.checkers.pop_count() {
            2 => {
                ///////////////////////////////////////////////////////////////////
                // If we're in a double check, we have to move the king. We've already
                // generated those moves, so just return.
                ///////////////////////////////////////////////////////////////////
                return;
            }
            1 => {
                let checker_square = self.checkers.lsb();
                let pt = self.piece_at(checker_square).type_of();
                match pt {
                    PieceType::Pawn | PieceType::Knight => {
                        ///////////////////////////////////////////////////////////////////
                        // If the checkers is a pawn, we have to look out for ep moves
                        // that can capture it.
                        ///////////////////////////////////////////////////////////////////
                        if pt == PieceType::Pawn
                            && self.checkers
                                == self.history[self.game_ply]
                                    .epsq()
                                    .bb()
                                    .shift(Direction::South.relative(us), 1)
                        {
                            b1 = attacks::pawn_attacks_sq(self.history[self.game_ply].epsq(), them)
                                & self.bitboard_of(us, PieceType::Pawn)
                                & not_pinned;
                            for sq in b1 {
                                moves.push(Move::new(
                                    sq,
                                    self.history[self.game_ply].epsq(),
                                    MoveFlags::EnPassant,
                                ));
                            }
                        }
                        b1 = self.attackers_from_color(checker_square, all, us) & not_pinned;
                        for sq in b1 {
                            if self.piece_type_at(sq) == PieceType::Pawn
                                && sq.rank().relative(us) == Rank::Seven
                            {
                                moves.push(Move::new(sq, checker_square, MoveFlags::PcQueen));
                                moves.push(Move::new(sq, checker_square, MoveFlags::PcRook));
                                moves.push(Move::new(sq, checker_square, MoveFlags::PcKnight));
                                moves.push(Move::new(sq, checker_square, MoveFlags::PcBishop));
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
                        capture_mask = self.checkers;
                        quiet_mask = BitBoard::between(our_king, checker_square);
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
                if self.history[self.game_ply].epsq() != SQ::None {
                    b2 = attacks::pawn_attacks_sq(self.history[self.game_ply].epsq(), them)
                        & self.bitboard_of(us, PieceType::Pawn);
                    b1 = b2 & not_pinned;
                    for sq in b1 {
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
                            all ^ sq.bb()
                                ^ self.history[self.game_ply]
                                    .epsq()
                                    .bb()
                                    .shift(Direction::South.relative(us), 1),
                            our_king.rank().bb(),
                        );

                        if (attacks & their_orth_sliders) == BitBoard::ZERO {
                            moves.push(Move::new(
                                sq,
                                self.history[self.game_ply].epsq(),
                                MoveFlags::EnPassant,
                            ));
                        }
                    }
                    ///////////////////////////////////////////////////////////////////
                    // Pinned pawns can only capture ep if they are pinned diagonally
                    // and the ep square is in line with the king.
                    ///////////////////////////////////////////////////////////////////
                    b1 = b2
                        & self.pinned
                        & BitBoard::line(self.history[self.game_ply].epsq(), our_king);
                    if b1 != BitBoard::ZERO {
                        moves.push(Move::new(
                            b1.lsb(),
                            self.history[self.game_ply].epsq(),
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
                if ((self.history[self.game_ply].entry() & BitBoard::oo_mask(us))
                    | ((all | danger) & BitBoard::oo_blockers_mask(us)))
                    == BitBoard::ZERO
                {
                    moves.push(if us == Color::White {
                        Move::new(SQ::E1, SQ::G1, MoveFlags::OO)
                    } else {
                        Move::new(SQ::E8, SQ::G8, MoveFlags::OO)
                    })
                }
                if ((self.history[self.game_ply].entry() & BitBoard::ooo_mask(us))
                    | ((all | (danger & !BitBoard::ignore_ooo_danger(us)))
                        & BitBoard::ooo_blockers_mask(us)))
                    == BitBoard::ZERO
                {
                    moves.push(if us == Color::White {
                        Move::new(SQ::E1, SQ::C1, MoveFlags::OOO)
                    } else {
                        Move::new(SQ::E8, SQ::C8, MoveFlags::OOO)
                    })
                }
                ///////////////////////////////////////////////////////////////////
                // For each pinned rook, bishop, or queen, only include attacks
                // that are aligned with our king.
                ///////////////////////////////////////////////////////////////////
                b1 = !(not_pinned | self.bitboard_of(us, PieceType::Knight));
                for sq in b1 {
                    b2 = attacks::attacks(self.piece_type_at(sq), sq, all)
                        & BitBoard::line(our_king, sq);
                    moves.make_q(sq, b2 & quiet_mask);
                    moves.make_c(sq, b2 & capture_mask);
                }

                ///////////////////////////////////////////////////////////////////
                // For each pinned pawn
                ///////////////////////////////////////////////////////////////////
                b1 = !not_pinned & self.bitboard_of(us, PieceType::Pawn);
                for sq in b1 {
                    ///////////////////////////////////////////////////////////////////
                    // Quiet promotions are impossible since the square in front of the
                    // pawn will be occupied
                    ///////////////////////////////////////////////////////////////////
                    if sq.rank() == Rank::Seven.relative(us) {
                        b2 = attacks::pawn_attacks_sq(sq, us)
                            & capture_mask
                            & BitBoard::line(our_king, sq);
                        moves.make_pc(sq, b2);
                    } else {
                        b2 = attacks::pawn_attacks_sq(sq, us)
                            & them_bb
                            & BitBoard::line(sq, our_king);
                        moves.make_c(sq, b2);

                        ///////////////////////////////////////////////////////////////////
                        // Single and double pawn pushes
                        ///////////////////////////////////////////////////////////////////
                        b2 = sq.bb().shift(Direction::North.relative(us), 1)
                            & !all
                            & BitBoard::line(our_king, sq);
                        b3 = (b2 & Rank::Three.relative(us).bb())
                            .shift(Direction::North.relative(us), 1)
                            & !all
                            & BitBoard::line(our_king, sq);

                        moves.make_q(sq, b2);
                        moves.make_dp(sq, b3);
                    }
                }
            }
        }

        ///////////////////////////////////////////////////////////////////
        // Non-pinned moves from here
        ///////////////////////////////////////////////////////////////////
        b1 = self.bitboard_of(us, PieceType::Knight) & not_pinned;
        for sq in b1 {
            b2 = attacks::knight_attacks(sq);
            moves.make_c(sq, b2 & capture_mask);
            moves.make_q(sq, b2 & quiet_mask);
        }

        b1 = our_diag_sliders & not_pinned;
        for sq in b1 {
            b2 = attacks::bishop_attacks(sq, all);
            moves.make_c(sq, b2 & capture_mask);
            moves.make_q(sq, b2 & quiet_mask);
        }

        b1 = our_orth_sliders & not_pinned;
        for sq in b1 {
            b2 = attacks::rook_attacks(sq, all);
            moves.make_c(sq, b2 & capture_mask);
            moves.make_q(sq, b2 & quiet_mask);
        }

        b1 = self.bitboard_of(us, PieceType::Pawn) & not_pinned & !Rank::Seven.relative(us).bb();
        b2 = b1.shift(Direction::North.relative(us), 1) & !all;
        b3 = (b2 & Rank::Three.relative(us).bb()).shift(Direction::North.relative(us), 1)
            & quiet_mask;

        b2 &= quiet_mask;

        for sq in b2 {
            moves.push(Move::new(
                sq - Direction::North.relative(us),
                sq,
                MoveFlags::Quiet,
            ));
        }

        for sq in b3 {
            moves.push(Move::new(
                sq - Direction::NorthNorth.relative(us),
                sq,
                MoveFlags::DoublePush,
            ));
        }

        b2 = b1.shift(Direction::NorthWest.relative(us), 1) & capture_mask;
        b3 = b1.shift(Direction::NorthEast.relative(us), 1) & capture_mask;

        for sq in b2 {
            moves.push(Move::new(
                sq - Direction::NorthWest.relative(us),
                sq,
                MoveFlags::Capture,
            ));
        }

        for sq in b3 {
            moves.push(Move::new(
                sq - Direction::NorthEast.relative(us),
                sq,
                MoveFlags::Capture,
            ));
        }

        b1 = self.bitboard_of(us, PieceType::Pawn) & not_pinned & Rank::Seven.relative(us).bb();

        if b1 != BitBoard::ZERO {
            b2 = b1.shift(Direction::North.relative(us), 1) & quiet_mask;
            for sq in b2 {
                moves.push(Move::new(
                    sq - Direction::North.relative(us),
                    sq,
                    MoveFlags::PrQueen,
                ));
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

            b2 = b1.shift(Direction::NorthWest.relative(us), 1) & capture_mask;
            b3 = b1.shift(Direction::NorthEast.relative(us), 1) & capture_mask;
            for sq in b2 {
                moves.push(Move::new(
                    sq - Direction::NorthWest.relative(us),
                    sq,
                    MoveFlags::PcQueen,
                ));
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

            for sq in b3 {
                moves.push(Move::new(
                    sq - Direction::NorthEast.relative(us),
                    sq,
                    MoveFlags::PcQueen,
                ));
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

    pub fn generate_legal_q_moves(&mut self, moves: &mut MoveList) {
        let us = self.color_to_play;
        let them = !self.color_to_play;

        let us_bb = self.all_pieces_color(us);
        let them_bb = self.all_pieces_color(them);
        let all = us_bb | them_bb;

        let our_king = self
            .bitboard_of_piece(Piece::make_piece(us, PieceType::King))
            .lsb();
        let their_king = self
            .bitboard_of_piece(Piece::make_piece(them, PieceType::King))
            .lsb();

        let our_diag_sliders = self.diagonal_sliders(us);
        let their_diag_sliders = self.diagonal_sliders(them);
        let our_orth_sliders = self.orthogonal_sliders(us);
        let their_orth_sliders = self.orthogonal_sliders(them);

        let mut b1: BitBoard;
        let mut b2: BitBoard;
        let mut b3: BitBoard;

        let mut danger = BitBoard::ZERO;

        danger |= attacks::pawn_attacks_bb(self.bitboard_of(them, PieceType::Pawn), them)
            | attacks::king_attacks(their_king);

        b1 = self.bitboard_of(them, PieceType::Knight);
        for sq in b1 {
            danger |= attacks::knight_attacks(sq);
        }

        b1 = their_diag_sliders;
        for sq in b1 {
            danger |= attacks::bishop_attacks(sq, all ^ our_king.bb());
        }

        b1 = their_orth_sliders;
        for sq in b1 {
            danger |= attacks::rook_attacks(sq, all ^ our_king.bb());
        }

        let king_attacks = attacks::king_attacks(our_king) & !(us_bb | danger);
        moves.make_c(our_king, king_attacks & them_bb);

        let capture_mask: BitBoard;
        let quiet_mask: BitBoard;

        self.checkers = (attacks::knight_attacks(our_king)
            & self.bitboard_of(them, PieceType::Knight))
            | (attacks::pawn_attacks_sq(our_king, us) & self.bitboard_of(them, PieceType::Pawn));

        let candidates = (attacks::rook_attacks(our_king, them_bb) & their_orth_sliders)
            | (attacks::bishop_attacks(our_king, them_bb) & their_diag_sliders);

        self.pinned = BitBoard::ZERO;
        for sq in candidates {
            b1 = BitBoard::between(our_king, sq) & us_bb;
            if b1 == BitBoard::ZERO {
                self.checkers ^= sq.bb();
            } else if b1.is_single() {
                self.pinned ^= b1;
            }
        }

        let not_pinned = !self.pinned;

        match self.checkers.pop_count() {
            2 => {
                moves.make_q(our_king, king_attacks & !them_bb);
                return;
            }
            1 => {
                let checker_square = self.checkers.lsb();
                let pt = self.piece_at(checker_square).type_of();
                match pt {
                    PieceType::Pawn | PieceType::Knight => {
                        ///////////////////////////////////////////////////////////////////
                        // If the checkers is a pawn, we have to look out for ep moves
                        // that can capture it.
                        ///////////////////////////////////////////////////////////////////
                        if pt == PieceType::Pawn
                            && self.checkers
                                == self.history[self.game_ply]
                                    .epsq()
                                    .bb()
                                    .shift(Direction::South.relative(us), 1)
                        {
                            b1 = attacks::pawn_attacks_sq(self.history[self.game_ply].epsq(), them)
                                & self.bitboard_of(us, PieceType::Pawn)
                                & not_pinned;
                            for sq in b1 {
                                moves.push(Move::new(
                                    sq,
                                    self.history[self.game_ply].epsq(),
                                    MoveFlags::EnPassant,
                                ));
                            }
                        }
                        b1 = self.attackers_from_color(checker_square, all, us) & not_pinned;
                        for sq in b1 {
                            if self.piece_type_at(sq) == PieceType::Pawn
                                && sq.rank().relative(us) == Rank::Seven
                            {
                                moves.push(Move::new(sq, checker_square, MoveFlags::PcQueen));
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
                        capture_mask = self.checkers;
                        quiet_mask = BitBoard::between(our_king, checker_square);
                    }
                }
            }
            _ => {
                capture_mask = them_bb;
                quiet_mask = !all;
                if self.history[self.game_ply].epsq() != SQ::None {
                    b2 = attacks::pawn_attacks_sq(self.history[self.game_ply].epsq(), them)
                        & self.bitboard_of(us, PieceType::Pawn);
                    b1 = b2 & not_pinned;
                    for sq in b1 {
                        let attacks = attacks::sliding_attacks(
                            our_king,
                            all ^ sq.bb()
                                ^ self.history[self.game_ply]
                                    .epsq()
                                    .bb()
                                    .shift(Direction::South.relative(us), 1),
                            our_king.rank().bb(),
                        );

                        if (attacks & their_orth_sliders) == BitBoard::ZERO {
                            moves.push(Move::new(
                                sq,
                                self.history[self.game_ply].epsq(),
                                MoveFlags::EnPassant,
                            ));
                        }
                    }
                    b1 = b2
                        & self.pinned
                        & BitBoard::line(self.history[self.game_ply].epsq(), our_king);
                    if b1 != BitBoard::ZERO {
                        moves.push(Move::new(
                            b1.lsb(),
                            self.history[self.game_ply].epsq(),
                            MoveFlags::EnPassant,
                        ));
                    }
                }

                b1 = !(not_pinned | self.bitboard_of(us, PieceType::Knight));
                for sq in b1 {
                    b2 = attacks::attacks(self.piece_type_at(sq), sq, all)
                        & BitBoard::line(our_king, sq);
                    moves.make_c(sq, b2 & capture_mask);
                }

                b1 = !not_pinned & self.bitboard_of(us, PieceType::Pawn);
                for from_sq in b1 {
                    if from_sq.rank() == Rank::Seven.relative(us) {
                        b2 = attacks::pawn_attacks_sq(from_sq, us)
                            & capture_mask
                            & BitBoard::line(our_king, from_sq);
                        for to_sq in b2 {
                            moves.push(Move::new(from_sq, to_sq, MoveFlags::PcQueen))
                        }
                    }
                }
            }
        }

        b1 = self.bitboard_of(us, PieceType::Knight) & not_pinned;
        for sq in b1 {
            b2 = attacks::knight_attacks(sq);
            moves.make_c(sq, b2 & capture_mask);
        }

        b1 = our_diag_sliders & not_pinned;
        for sq in b1 {
            b2 = attacks::bishop_attacks(sq, all);
            moves.make_c(sq, b2 & capture_mask);
        }

        b1 = our_orth_sliders & not_pinned;
        for sq in b1 {
            b2 = attacks::rook_attacks(sq, all);
            moves.make_c(sq, b2 & capture_mask);
        }

        b1 = self.bitboard_of(us, PieceType::Pawn) & not_pinned & !Rank::Seven.relative(us).bb();
        b2 = b1.shift(Direction::NorthWest.relative(us), 1) & capture_mask;
        b3 = b1.shift(Direction::NorthEast.relative(us), 1) & capture_mask;

        for sq in b2 {
            moves.push(Move::new(
                sq - Direction::NorthWest.relative(us),
                sq,
                MoveFlags::Capture,
            ));
        }

        for sq in b3 {
            moves.push(Move::new(
                sq - Direction::NorthEast.relative(us),
                sq,
                MoveFlags::Capture,
            ));
        }

        b1 = self.bitboard_of(us, PieceType::Pawn) & not_pinned & Rank::Seven.relative(us).bb();
        if b1 != BitBoard::ZERO {
            b2 = b1.shift(Direction::North.relative(us), 1) & quiet_mask;
            for sq in b2 {
                moves.push(Move::new(
                    sq - Direction::North.relative(us),
                    sq,
                    MoveFlags::PrQueen,
                ));
            }

            b2 = b1.shift(Direction::NorthWest.relative(us), 1) & capture_mask;
            b3 = b1.shift(Direction::NorthEast.relative(us), 1) & capture_mask;
            for sq in b2 {
                moves.push(Move::new(
                    sq - Direction::NorthWest.relative(us),
                    sq,
                    MoveFlags::PcQueen,
                ));
            }

            for sq in b3 {
                moves.push(Move::new(
                    sq - Direction::NorthEast.relative(us),
                    sq,
                    MoveFlags::PcQueen,
                ));
            }
        }
    }

    // Set fen function from pabi
    pub fn set_fen(&mut self, fen: &str) -> Result<Self, &str> {
        let fen = fen.trim();
        if !fen.is_ascii() || fen.lines().count() != 1 {
            return Err("FEN should be a single ASCII line.");
        }
        self.clear();
        let mut parts = fen.split_ascii_whitespace();

        if parts.clone().count() < 3 {
            return Err(
                "Fen must at include at least piece placement, color, and castling string.",
            );
        }

        let pieces_placement = parts.next().unwrap();
        let color_to_play = parts.next().unwrap();
        let castling_ability = parts.next().unwrap();
        let en_passant_square = parts.next().unwrap_or("-");
        let halfmove_clock = parts.next().unwrap_or("0").parse::<u16>().unwrap();
        let fullmove_counter = if let Ok(fullmove_number) = parts.next().unwrap_or("1").parse::<usize>() {
            if fullmove_number > 0 {
                fullmove_number
            } else {
                fullmove_number + 1
            }
        } else {
            1
        };

        if pieces_placement.split("/").count() != 8 {
            return Err("Pieces Placement FEN should have 8 ranks.");
        }

        self.color_to_play = if color_to_play == "w" {
            Color::White
        } else if color_to_play == "b" {
            Color::Black
        } else {
            return Err("Color to play should be either w or b");
        };

        if self.color_to_play == Color::Black {
            self.hash ^= zobrist::zobrist_color();
        }

        self.game_ply = (fullmove_counter - 1) * 2;

        let ranks = pieces_placement.split("/");
        for (rank_idx, rank_fen) in ranks.enumerate() {
            let mut idx = (7 - rank_idx) * 8;

            for ch in rank_fen.chars() {
                let digit = ch.to_digit(10);
                if let Some(digit) = digit {
                    idx += digit as usize;
                } else {
                    let sq = SQ::from(idx as u8);
                    match Piece::try_from(ch) {
                        Ok(piece) => self.set_piece_at(piece, sq),
                        Err(_e) => return Err("FEN has incorrect piece symbol."),
                    }

                    idx += 1;
                }
            }
        }

        for (symbol, mask) in "KQkq".chars().zip([
            BitBoard::WHITE_OO_MASK,
            BitBoard::WHITE_OOO_MASK,
            BitBoard::BLACK_OO_MASK,
            BitBoard::BLACK_OOO_MASK,
        ]) {
            if !castling_ability.contains(symbol) {
                self.history[self.game_ply].set_entry(self.history[self.game_ply].entry() | mask);
            }
        }

        if en_passant_square != "-" {
            if SQ_DISPLAY.contains(&en_passant_square) {
                let sq = SQ::from(en_passant_square);
                self.history[self.game_ply].set_epsq(sq);
                self.hash ^= zobrist::zobrist_ep(sq.file());
            } else {
                return Err("En Passant square is not valid");
            }
        }

        self.history[self.game_ply].set_half_move_counter(halfmove_clock);
        Ok(*self)
    }

    pub fn push_str(&mut self, move_str: String) -> Result<Move, &'static str> {
        let from_sq = SQ::from(&move_str[..2]);
        let to_sq = SQ::from(&move_str[2..4]);

        let promo: Option<PieceType>;

        if move_str.len() > 4 {
            promo = match Piece::try_from(move_str.chars().nth(4).unwrap()) {
                Ok(piece) => Some(piece.type_of()),
                Err(e) => return Err(e),
            };
        } else {
            promo = None;
        }

        let mut m = Move::NULL;
        if self.piece_at(to_sq) != Piece::None {
            match promo {
                Some(PieceType::Queen) => {
                    m = Move::new(from_sq, to_sq, MoveFlags::PcQueen);
                }
                Some(PieceType::Knight) => {
                    m = Move::new(from_sq, to_sq, MoveFlags::PcKnight);
                }
                Some(PieceType::Bishop) => {
                    m = Move::new(from_sq, to_sq, MoveFlags::PcBishop);
                }
                Some(PieceType::Rook) => {
                    m = Move::new(from_sq, to_sq, MoveFlags::PcRook);
                }
                None => {
                    m = Move::new(from_sq, to_sq, MoveFlags::Capture);
                }
                _ => {}
            }
        } else {
            match promo {
                Some(PieceType::Queen) => {
                    m = Move::new(from_sq, to_sq, MoveFlags::PrQueen);
                }
                Some(PieceType::Knight) => {
                    m = Move::new(from_sq, to_sq, MoveFlags::PrKnight);
                }
                Some(PieceType::Bishop) => {
                    m = Move::new(from_sq, to_sq, MoveFlags::PrBishop);
                }
                Some(PieceType::Rook) => {
                    m = Move::new(from_sq, to_sq, MoveFlags::PrRook);
                }
                None => {
                    if self.piece_type_at(from_sq) == PieceType::Pawn
                        && to_sq == self.history[self.game_ply].epsq()
                    {
                        m = Move::new(from_sq, to_sq, MoveFlags::EnPassant);
                    } else if self.piece_type_at(from_sq) == PieceType::Pawn
                        && i8::abs(from_sq as i8 - to_sq as i8) == 16
                    {
                        m = Move::new(from_sq, to_sq, MoveFlags::DoublePush);
                    } else if self.piece_type_at(from_sq) == PieceType::King
                        && from_sq.file() == File::E
                        && to_sq.file() == File::G
                    {
                        m = Move::new(from_sq, to_sq, MoveFlags::OO);
                    } else if self.piece_type_at(from_sq) == PieceType::King
                        && from_sq.file() == File::E
                        && to_sq.file() == File::C
                    {
                        m = Move::new(from_sq, to_sq, MoveFlags::OOO);
                    } else {
                        m = Move::new(from_sq, to_sq, MoveFlags::Quiet);
                    }
                }
                _ => {}
            }
        }
        self.push(m);
        Ok(m)
    }

    pub fn fen(&self) -> String {
        let mut board_string = String::new();
        for rank_idx in (0..=7).rev() {
            let rank = Rank::from(rank_idx);
            let mut empty_squares = 0;
            for file_idx in 0..=7 {
                let file = File::from(file_idx);
                let sq = SQ::encode(rank, file);
                let pc = self.board[sq.index()];
                if pc != Piece::None {
                    if empty_squares != 0 {
                        board_string.push_str(format!("{}", empty_squares).as_str());
                        empty_squares = 0;
                    }
                    board_string.push(pc.uci());
                } else {
                    empty_squares += 1;
                }
            }
            if empty_squares != 0 {
                board_string.push_str(format!("{}", empty_squares).as_str());
            }
            if rank != Rank::One {
                board_string.push('/');
            }
        }

        let color_to_play = if self.color_to_play == Color::White {
            "w"
        } else {
            "b"
        };

        let mut castling_rights = String::new();
        for (symbol, mask) in "KQkq".chars().zip([
            BitBoard::WHITE_OO_MASK,
            BitBoard::WHITE_OOO_MASK,
            BitBoard::BLACK_OO_MASK,
            BitBoard::BLACK_OOO_MASK,
        ]) {
            if mask & self.history[self.game_ply].entry() == BitBoard::ZERO {
                castling_rights.push(symbol);
            }
        }
        if castling_rights.is_empty() {
            castling_rights = "-".to_string();
        }

        let epsq = if self.history[self.game_ply].epsq() != SQ::None {
            self.history[self.game_ply].epsq().to_string()
        } else {
            "-".to_string()
        };

        format!(
            "{} {} {} {} {} {}",
            board_string,
            color_to_play,
            castling_rights,
            epsq,
            self.history[self.game_ply].half_move_counter(),
            self.game_ply / 2 + 1,
        )
    }

    #[inline(always)]
    pub fn color_to_play(&self) -> Color {
        self.color_to_play
    }

    #[inline(always)]
    pub fn game_ply(&self) -> usize {
        self.game_ply
    }

    #[inline(always)]
    pub fn checkers(&self) -> BitBoard {
        self.checkers
    }

    #[inline(always)]
    pub fn material_score(&self) -> Score {
        self.material_score
    }

    #[inline(always)]
    pub fn p_sq_score(&self) -> Score {
        self.p_sq_score
    }

    #[inline(always)]
    pub fn phase(&self) -> Phase {
        self.phase
    }

    #[inline(always)]
    pub fn hash(&self) -> Hash {
        self.hash
    }

    #[inline(always)]
    pub fn material_hash(&self) -> Hash {
        self.material_hash
    }
}

impl From<&str> for Board {
    fn from(fen: &str) -> Self {
        let mut board = Self::new();
        return match board.set_fen(fen) {
            Ok(board) => board,
            Err(e) => {
                println!("{}", e);
                Board::new()
            }
        };
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::with_capacity(N_SQUARES * 2 + 8);
        for rank_idx in (0..=7).rev() {
            let rank = Rank::from(rank_idx);
            for file_idx in 0..=7 {
                let file = File::from(file_idx);
                let sq = SQ::encode(rank, file);
                let pc = self.piece_at(sq);
                let char = if pc != Piece::None { pc.uci() } else { '-' };
                s.push(char);
                s.push(' ');
                if sq.file() == File::H {
                    s.push('\n');
                }
            }
        }
        write!(f, "{}", s)
    }
}
