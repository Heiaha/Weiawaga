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
use std::fmt;

pub struct Board {
    piece_bb: [BitBoard; N_PIECES],
    board: [Piece; N_SQUARES],

    color_to_play: Color,

    hash: Key,
    material_hash: Key,
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
        board.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        board
    }

    pub fn clean() -> Self {
        Board {
            piece_bb: [BitBoard::ZERO; N_PIECES],
            board: [Piece::None; N_SQUARES],
            color_to_play: Color::White,
            hash: BitBoard::ZERO,
            material_hash: BitBoard::ZERO,
            game_ply: 0,
            phase: Score::TOTAL_PHASE,
            material_score: Score::ZERO,
            p_sq_score: Score::ZERO,
            history: [UndoInfo::empty(); 1000],
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
        self.history = [UndoInfo::empty(); 1000];
        self.checkers = BitBoard::ZERO;
        self.pinned = BitBoard::ZERO;

        for pc in Piece::WhitePawn..=Piece::BlackKing {
            self.piece_bb[pc.index()] = BitBoard::ZERO;
        }

        for sq in SQ::A1..=SQ::H8 {
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
        self.material_score += e_constants::piece_value(pc);

        self.board[sq.index()] = pc;
        self.piece_bb[pc.index()] |= sq.bb();

        self.hash ^= zobrist::zobrist_table(pc, sq);
        self.material_hash ^= zobrist::zobrist_table(pc, sq);
    }

    pub fn remove_piece(&mut self, sq: SQ) {
        let pc = self.piece_at(sq);
        self.phase += Score::piece_phase(pc.type_of());
        self.p_sq_score -= e_constants::piece_sq_value(pc, sq);
        self.material_score -= e_constants::piece_value(pc);

        self.hash ^= zobrist::zobrist_table(pc, sq);
        self.material_hash ^= zobrist::zobrist_table(pc, sq);

        self.piece_bb[pc.index()] &= !sq.bb();
        self.board[sq.index()] = Piece::None;
    }

    pub fn move_piece_quiet(&mut self, from_sq: SQ, to_sq: SQ) {
        let pc = self.piece_at(from_sq);
        self.p_sq_score +=
            e_constants::piece_sq_value(pc, to_sq) - e_constants::piece_sq_value(pc, from_sq);

        let hash_update = zobrist::zobrist_table(pc, from_sq) ^ zobrist::zobrist_table(pc, to_sq);
        self.hash ^= hash_update;
        self.material_hash ^= hash_update;

        self.piece_bb[pc.index()] ^= from_sq.bb() | to_sq.bb();
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

    pub fn diagonal_sliders(&self, color: Color) -> BitBoard {
        if color == Color::White {
            self.piece_bb[Piece::WhiteBishop.index()] | self.piece_bb[Piece::WhiteQueen.index()]
        } else {
            self.piece_bb[Piece::BlackBishop.index()] | self.piece_bb[Piece::BlackQueen.index()]
        }
    }

    pub fn orthogonal_sliders(&self, color: Color) -> BitBoard {
        if color == Color::White {
            self.piece_bb[Piece::WhiteRook.index()] | self.piece_bb[Piece::WhiteQueen.index()]
        } else {
            self.piece_bb[Piece::BlackRook.index()] | self.piece_bb[Piece::BlackQueen.index()]
        }
    }

    pub fn all_pieces(&self, color: Color) -> BitBoard {
        if color == Color::White {
            self.piece_bb[Piece::WhitePawn.index()]
                | self.piece_bb[Piece::WhiteKnight.index()]
                | self.piece_bb[Piece::WhiteBishop.index()]
                | self.piece_bb[Piece::WhiteRook.index()]
                | self.piece_bb[Piece::WhiteQueen.index()]
                | self.piece_bb[Piece::WhiteKing.index()]
        } else {
            self.piece_bb[Piece::BlackPawn.index()]
                | self.piece_bb[Piece::BlackKnight.index()]
                | self.piece_bb[Piece::BlackBishop.index()]
                | self.piece_bb[Piece::BlackRook.index()]
                | self.piece_bb[Piece::BlackQueen.index()]
                | self.piece_bb[Piece::BlackKing.index()]
        }
    }

    pub fn attackers_from(&self, sq: SQ, occ: BitBoard, color: Color) -> BitBoard {
        if color == Color::White {
            (self.piece_bb[Piece::WhitePawn.index()] & attacks::pawn_attacks_sq(sq, Color::Black))
                | (self.piece_bb[Piece::WhiteKnight.index()] & attacks::knight_attacks(sq))
                | (self.piece_bb[Piece::WhiteBishop.index()] & attacks::bishop_attacks(sq, occ))
                | (self.piece_bb[Piece::WhiteRook.index()] & attacks::rook_attacks(sq, occ))
                | (self.piece_bb[Piece::WhiteQueen.index()]
                    & (attacks::bishop_attacks(sq, occ) | attacks::rook_attacks(sq, occ)))
        } else {
            (self.piece_bb[Piece::BlackPawn.index()] & attacks::pawn_attacks_sq(sq, Color::White))
                | (self.piece_bb[Piece::BlackKnight.index()] & attacks::knight_attacks(sq))
                | (self.piece_bb[Piece::BlackBishop.index()] & attacks::bishop_attacks(sq, occ))
                | (self.piece_bb[Piece::BlackRook.index()] & attacks::rook_attacks(sq, occ))
                | (self.piece_bb[Piece::BlackQueen.index()]
                    & (attacks::bishop_attacks(sq, occ) | attacks::rook_attacks(sq, occ)))
        }
    }

    pub fn king_attacked(&self) -> bool {
        let us = self.color_to_play;
        let them = !self.color_to_play;
        let our_king = self.bitboard_of(us, PieceType::King).lsb();

        if attacks::pawn_attacks_sq(our_king, us) & self.bitboard_of(them, PieceType::Pawn)
            != BitBoard::ZERO
        {
            return true;
        }

        if attacks::knight_attacks(our_king) & self.bitboard_of(them, PieceType::Knight)
            != BitBoard::ZERO
        {
            return true;
        }

        let all = self.all_pieces(us) | self.all_pieces(them);

        let their_diag_sliders = self.diagonal_sliders(them);
        let their_orth_sliders = self.orthogonal_sliders(them);

        if attacks::rook_attacks(our_king, all) & their_orth_sliders != BitBoard::ZERO {
            return true;
        }

        if attacks::bishop_attacks(our_king, all) & their_diag_sliders != BitBoard::ZERO {
            return true;
        }
        false
    }

    pub fn is_repetition_or_fifty(&self) -> bool {
        if self.history[self.game_ply].half_move_counter() >= 100 {
            return true;
        }
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

    pub fn has_non_pawn_material(&self) -> bool {
        for pc in PieceType::Knight..=PieceType::Queen {
            if self.bitboard_of(self.color_to_play, pc) != BitBoard::ZERO {
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

        let us_bb = self.all_pieces(us);
        let them_bb = self.all_pieces(them);
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

        b1 = attacks::king_attacks(our_king) & !(us_bb | danger);

        moves.make_q(our_king, b1 & !them_bb);
        moves.make_c(our_king, b1 & them_bb);

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
                return;
            }
            1 => {
                let checker_square = self.checkers.lsb();
                match self.piece_at(checker_square).type_of() {
                    PieceType::Pawn => {
                        if self.checkers
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
                        b1 = self.attackers_from(checker_square, all, us) & not_pinned;
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
                    PieceType::Knight => {
                        b1 = self.attackers_from(checker_square, all, us) & not_pinned;
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

                b1 = !(not_pinned | self.bitboard_of(us, PieceType::Knight));
                for sq in b1 {
                    b2 = attacks::attacks(self.piece_type_at(sq), sq, all)
                        & BitBoard::line(our_king, sq);
                    moves.make_q(sq, b2 & quiet_mask);
                    moves.make_c(sq, b2 & capture_mask);
                }

                b1 = !not_pinned & self.bitboard_of(us, PieceType::Pawn);
                for sq in b1 {
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

        let us_bb = self.all_pieces(us);
        let them_bb = self.all_pieces(them);
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

        b1 = attacks::king_attacks(our_king) & !(us_bb | danger);

        moves.make_q(our_king, b1 & !them_bb);
        moves.make_c(our_king, b1 & them_bb);

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
                return;
            }
            1 => {
                let checker_square = self.checkers.lsb();
                match self.piece_at(checker_square).type_of() {
                    PieceType::Pawn => {
                        if self.checkers
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
                        b1 = self.attackers_from(checker_square, all, us) & not_pinned;
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
                    PieceType::Knight => {
                        b1 = self.attackers_from(checker_square, all, us) & not_pinned;
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
                for sq in b1 {
                    if sq.rank() == Rank::Seven.relative(us) {
                        b2 = attacks::pawn_attacks_sq(sq, us)
                            & capture_mask
                            & BitBoard::line(our_king, sq);
                        moves.make_pc(sq, b2);
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

    pub fn set_fen(&mut self, fen: &str) {
        self.clear();
        let det_split: Vec<&str> = fen.split_whitespace().collect();
        let squares: Vec<&str> = det_split[0].split('/').collect();
        let ranks = squares.as_slice();

        if det_split.len() > 5 {
            let full_move_number = det_split[5].parse::<usize>().unwrap();
            self.game_ply = (full_move_number - 1) * 2;
            if self.color_to_play == Color::Black {
                self.game_ply += 1;
            }
        }

        for (i, rank) in ranks.iter().enumerate() {
            let mut idx = (7 - i) * 8;

            for ch in rank.chars() {
                let dig = ch.to_digit(10);
                if let Some(digit) = dig {
                    idx += digit as usize;
                } else {
                    let sq = SQ::from(idx as u8);
                    match ch {
                        'P' => {
                            self.set_piece_at(Piece::WhitePawn, sq);
                        }
                        'N' => {
                            self.set_piece_at(Piece::WhiteKnight, sq);
                        }
                        'B' => {
                            self.set_piece_at(Piece::WhiteBishop, sq);
                        }
                        'R' => {
                            self.set_piece_at(Piece::WhiteRook, sq);
                        }
                        'Q' => {
                            self.set_piece_at(Piece::WhiteQueen, sq);
                        }
                        'K' => {
                            self.set_piece_at(Piece::WhiteKing, sq);
                        }
                        'p' => {
                            self.set_piece_at(Piece::BlackPawn, sq);
                        }
                        'n' => {
                            self.set_piece_at(Piece::BlackKnight, sq);
                        }
                        'b' => {
                            self.set_piece_at(Piece::BlackBishop, sq);
                        }
                        'r' => {
                            self.set_piece_at(Piece::BlackRook, sq);
                        }
                        'q' => {
                            self.set_piece_at(Piece::BlackQueen, sq);
                        }
                        'k' => {
                            self.set_piece_at(Piece::BlackKing, sq);
                        }
                        _ => {}
                    }
                    idx += 1;
                }
            }
        }
        self.color_to_play = if det_split[1] == "w" {
            Color::White
        } else {
            Color::Black
        };

        if self.color_to_play == Color::Black {
            self.hash ^= zobrist::zobrist_color();
        }
        if !det_split[2].contains('K') {
            self.history[self.game_ply]
                .set_entry(self.history[self.game_ply].entry() | BitBoard::WHITE_OO_MASK);
        }
        if !det_split[2].contains('Q') {
            self.history[self.game_ply]
                .set_entry(self.history[self.game_ply].entry() | BitBoard::WHITE_OOO_MASK);
        }
        if !det_split[2].contains('k') {
            self.history[self.game_ply]
                .set_entry(self.history[self.game_ply].entry() | BitBoard::BLACK_OO_MASK);
        }
        if !det_split[2].contains('q') {
            self.history[self.game_ply]
                .set_entry(self.history[self.game_ply].entry() | BitBoard::BLACK_OOO_MASK);
        }

        let s = det_split[3].to_ascii_lowercase();
        if s != "-" {
            let sq = SQ::from(s.trim());
            self.history[self.game_ply].set_epsq(sq);
            self.hash ^= zobrist::zobrist_ep(sq.file());
        }

        if det_split.len() > 4 {
            self.history[self.game_ply].set_half_move_counter(det_split[4].parse::<u16>().unwrap());
        }

        self.history[self.game_ply].set_material_hash(self.material_hash);
    }

    pub fn push_str(&mut self, move_str: String) {
        let from_sq = SQ::from(&move_str[..2]);
        let to_sq = SQ::from(&move_str[2..4]);

        let promo: Option<PieceType>;

        if move_str.len() > 4 {
            promo = Some(Piece::from(move_str.chars().nth(4).unwrap()).type_of());
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
    pub fn hash(&self) -> Key {
        self.hash
    }

    #[inline(always)]
    pub fn material_hash(&self) -> Key {
        self.material_hash
    }
}

impl From<&str> for Board {
    fn from(fen: &str) -> Self {
        let mut board = Self::new();
        board.set_fen(fen);
        board
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::with_capacity(N_SQUARES * 2 + 8);
        for sq in SQ_DISPLAY_ORDER {
            let op = self.piece_at(SQ::from(sq));
            let char = if op != Piece::None { op.uci() } else { '-' };
            s.push(char);
            s.push(' ');
            if sq % 8 == 7 {
                s.push('\n');
            }
        }
        write!(f, "{}", s)
    }
}
