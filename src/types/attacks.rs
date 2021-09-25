use super::bitboard::BitBoard;
use super::color::Color;
use super::magics::{BISHOP_MAGICS, ROOK_MAGICS};
use super::square::{Direction, SQ, N_SQUARES};
use crate::types::piece::PieceType;
use crate::types::color::N_COLORS;

const WHITE_PAWN_ATTACKS: [BitBoard; N_SQUARES] = [
    B!(0x0000000000000200), B!(0x0000000000000500), B!(0x0000000000000a00), B!(0x0000000000001400),
    B!(0x0000000000002800), B!(0x0000000000005000), B!(0x000000000000a000), B!(0x0000000000004000),
    B!(0x0000000000020000), B!(0x0000000000050000), B!(0x00000000000a0000), B!(0x0000000000140000),
    B!(0x0000000000280000), B!(0x0000000000500000), B!(0x0000000000a00000), B!(0x0000000000400000),
    B!(0x0000000002000000), B!(0x0000000005000000), B!(0x000000000a000000), B!(0x0000000014000000),
    B!(0x0000000028000000), B!(0x0000000050000000), B!(0x00000000a0000000), B!(0x0000000040000000),
    B!(0x0000000200000000), B!(0x0000000500000000), B!(0x0000000a00000000), B!(0x0000001400000000),
    B!(0x0000002800000000), B!(0x0000005000000000), B!(0x000000a000000000), B!(0x0000004000000000),
    B!(0x0000020000000000), B!(0x0000050000000000), B!(0x00000a0000000000), B!(0x0000140000000000),
    B!(0x0000280000000000), B!(0x0000500000000000), B!(0x0000a00000000000), B!(0x0000400000000000),
    B!(0x0002000000000000), B!(0x0005000000000000), B!(0x000a000000000000), B!(0x0014000000000000),
    B!(0x0028000000000000), B!(0x0050000000000000), B!(0x00a0000000000000), B!(0x0040000000000000),
    B!(0x0200000000000000), B!(0x0500000000000000), B!(0x0a00000000000000), B!(0x1400000000000000),
    B!(0x2800000000000000), B!(0x5000000000000000), B!(0xa000000000000000), B!(0x4000000000000000),
    B!(0x0000000000000000), B!(0x0000000000000000), B!(0x0000000000000000), B!(0x0000000000000000),
    B!(0x0000000000000000), B!(0x0000000000000000), B!(0x0000000000000000), B!(0x0000000000000000),
];

const BLACK_PAWN_ATTACKS: [BitBoard; N_SQUARES] = [
    B!(0x00000000000000), B!(0x00000000000000), B!(0x00000000000000), B!(0x00000000000000),
    B!(0x00000000000000), B!(0x00000000000000), B!(0x00000000000000), B!(0x00000000000000),
    B!(0x00000000000002), B!(0x00000000000005), B!(0x0000000000000a), B!(0x00000000000014),
    B!(0x00000000000028), B!(0x00000000000050), B!(0x000000000000a0), B!(0x00000000000040),
    B!(0x00000000000200), B!(0x00000000000500), B!(0x00000000000a00), B!(0x00000000001400),
    B!(0x00000000002800), B!(0x00000000005000), B!(0x0000000000a000), B!(0x00000000004000),
    B!(0x00000000020000), B!(0x00000000050000), B!(0x000000000a0000), B!(0x00000000140000),
    B!(0x00000000280000), B!(0x00000000500000), B!(0x00000000a00000), B!(0x00000000400000),
    B!(0x00000002000000), B!(0x00000005000000), B!(0x0000000a000000), B!(0x00000014000000),
    B!(0x00000028000000), B!(0x00000050000000), B!(0x000000a0000000), B!(0x00000040000000),
    B!(0x00000200000000), B!(0x00000500000000), B!(0x00000a00000000), B!(0x00001400000000),
    B!(0x00002800000000), B!(0x00005000000000), B!(0x0000a000000000), B!(0x00004000000000),
    B!(0x00020000000000), B!(0x00050000000000), B!(0x000a0000000000), B!(0x00140000000000),
    B!(0x00280000000000), B!(0x00500000000000), B!(0x00a00000000000), B!(0x00400000000000),
    B!(0x02000000000000), B!(0x05000000000000), B!(0x0a000000000000), B!(0x14000000000000),
    B!(0x28000000000000), B!(0x50000000000000), B!(0xa0000000000000), B!(0x40000000000000)
];

const KNIGHT_ATTACKS: [BitBoard; N_SQUARES] = [
    B!(0x0000000000020400), B!(0x0000000000050800), B!(0x00000000000a1100), B!(0x0000000000142200),
    B!(0x0000000000284400), B!(0x0000000000508800), B!(0x0000000000a01000), B!(0x0000000000402000),
    B!(0x0000000002040004), B!(0x0000000005080008), B!(0x000000000a110011), B!(0x0000000014220022),
    B!(0x0000000028440044), B!(0x0000000050880088), B!(0x00000000a0100010), B!(0x0000000040200020),
    B!(0x0000000204000402), B!(0x0000000508000805), B!(0x0000000a1100110a), B!(0x0000001422002214),
    B!(0x0000002844004428), B!(0x0000005088008850), B!(0x000000a0100010a0), B!(0x0000004020002040),
    B!(0x0000020400040200), B!(0x0000050800080500), B!(0x00000a1100110a00), B!(0x0000142200221400),
    B!(0x0000284400442800), B!(0x0000508800885000), B!(0x0000a0100010a000), B!(0x0000402000204000),
    B!(0x0002040004020000), B!(0x0005080008050000), B!(0x000a1100110a0000), B!(0x0014220022140000),
    B!(0x0028440044280000), B!(0x0050880088500000), B!(0x00a0100010a00000), B!(0x0040200020400000),
    B!(0x0204000402000000), B!(0x0508000805000000), B!(0x0a1100110a000000), B!(0x1422002214000000),
    B!(0x2844004428000000), B!(0x5088008850000000), B!(0xa0100010a0000000), B!(0x4020002040000000),
    B!(0x0400040200000000), B!(0x0800080500000000), B!(0x1100110a00000000), B!(0x2200221400000000),
    B!(0x4400442800000000), B!(0x8800885000000000), B!(0x100010a000000000), B!(0x2000204000000000),
    B!(0x0004020000000000), B!(0x0008050000000000), B!(0x00110a0000000000), B!(0x0022140000000000),
    B!(0x0044280000000000), B!(0x0088500000000000), B!(0x0010a00000000000), B!(0x0020400000000000)
];

const ADJACENT_ATTACKS: [BitBoard; N_SQUARES] = [
    B!(0x0000000000000302), B!(0x0000000000000705), B!(0x0000000000000e0a), B!(0x0000000000001c14),
    B!(0x0000000000003828), B!(0x0000000000007050), B!(0x000000000000e0a0), B!(0x000000000000c040),
    B!(0x0000000000030203), B!(0x0000000000070507), B!(0x00000000000e0a0e), B!(0x00000000001c141c),
    B!(0x0000000000382838), B!(0x0000000000705070), B!(0x0000000000e0a0e0), B!(0x0000000000c040c0),
    B!(0x0000000003020300), B!(0x0000000007050700), B!(0x000000000e0a0e00), B!(0x000000001c141c00),
    B!(0x0000000038283800), B!(0x0000000070507000), B!(0x00000000e0a0e000), B!(0x00000000c040c000),
    B!(0x0000000302030000), B!(0x0000000705070000), B!(0x0000000e0a0e0000), B!(0x0000001c141c0000),
    B!(0x0000003828380000), B!(0x0000007050700000), B!(0x000000e0a0e00000), B!(0x000000c040c00000),
    B!(0x0000030203000000), B!(0x0000070507000000), B!(0x00000e0a0e000000), B!(0x00001c141c000000),
    B!(0x0000382838000000), B!(0x0000705070000000), B!(0x0000e0a0e0000000), B!(0x0000c040c0000000),
    B!(0x0003020300000000), B!(0x0007050700000000), B!(0x000e0a0e00000000), B!(0x001c141c00000000),
    B!(0x0038283800000000), B!(0x0070507000000000), B!(0x00e0a0e000000000), B!(0x00c040c000000000),
    B!(0x0302030000000000), B!(0x0705070000000000), B!(0x0e0a0e0000000000), B!(0x1c141c0000000000),
    B!(0x3828380000000000), B!(0x7050700000000000), B!(0xe0a0e00000000000), B!(0xc040c00000000000),
    B!(0x0203000000000000), B!(0x0507000000000000), B!(0x0a0e000000000000), B!(0x141c000000000000),
    B!(0x2838000000000000), B!(0x5070000000000000), B!(0xa0e0000000000000), B!(0x40c0000000000000)
];

static mut PAWN_ATTACKS: [[BitBoard; N_SQUARES]; N_COLORS] = [[BitBoard::ZERO; N_SQUARES]; N_COLORS];

#[inline(always)]
pub fn rook_attacks(sq: SQ, occ: BitBoard) -> BitBoard {
    unsafe {
        ROOK_MAGICS.attacks[sq as usize][ROOK_MAGICS.index(sq, occ)]
    }
}

#[inline(always)]
pub fn bishop_attacks(sq: SQ, occ: BitBoard) -> BitBoard {
    unsafe {
        BISHOP_MAGICS.attacks[sq as usize][BISHOP_MAGICS.index(sq, occ)]
    }
}

#[inline(always)]
pub fn knight_attacks(sq: SQ) -> BitBoard {
    KNIGHT_ATTACKS[sq as usize]
}

#[inline(always)]
pub fn king_attacks(sq: SQ) -> BitBoard {
    ADJACENT_ATTACKS[sq as usize]
}

#[inline(always)]
pub fn pawn_attacks_bb(bb: BitBoard, color: Color) -> BitBoard {
    return if color == Color::White {
        bb.shift(Direction::NorthWest, 1) | bb.shift(Direction::NorthEast, 1)
    } else {
        bb.shift(Direction::SouthWest, 1) | bb.shift(Direction::SouthEast, 1)
    };
}

#[inline(always)]
pub fn pawn_attacks_sq(sq: SQ, color: Color) -> BitBoard {
    unsafe {
        PAWN_ATTACKS[color as usize][sq as usize]
    }
}

#[inline(always)]
pub fn sliding_attacks(sq: SQ, occ: BitBoard, mask: BitBoard) -> BitBoard {
    (((mask & occ) - sq.bb() * BitBoard::TWO)
        ^ ((mask & occ).reverse() - sq.bb().reverse() * BitBoard::TWO).reverse())
        & mask
}


pub fn attacks(pt: PieceType, sq: SQ, occ: BitBoard) -> BitBoard {
    match pt {
        PieceType::Pawn => { BitBoard::ZERO }
        PieceType::Knight => { knight_attacks(sq) }
        PieceType::Bishop => { bishop_attacks(sq, occ) }
        PieceType::Rook => { rook_attacks(sq, occ) }
        PieceType::Queen => { bishop_attacks(sq, occ) | rook_attacks(sq, occ) }
        PieceType::King => { king_attacks(sq) }
    }
}

//////////////////////////////////////////////
// Inits
//////////////////////////////////////////////

pub fn rook_attacks_for_init(sq: SQ, blockers: BitBoard) -> BitBoard {
    let mut attacks: BitBoard = BitBoard::ZERO;
    attacks |= sq.get_ray(Direction::North);
    if sq.get_ray(Direction::North) & blockers != BitBoard::ZERO {
        attacks &= !(sq.get_ray(Direction::North) & blockers)
            .lsb()
            .get_ray(Direction::North);
    }

    attacks |= sq.get_ray(Direction::South);
    if sq.get_ray(Direction::South) & blockers != BitBoard::ZERO {
        attacks &= !(sq.get_ray(Direction::South) & blockers)
            .msb()
            .get_ray(Direction::South);
    }

    attacks |= sq.get_ray(Direction::East);
    if sq.get_ray(Direction::East) & blockers != BitBoard::ZERO {
        attacks &= !(sq.get_ray(Direction::East) & blockers)
            .lsb()
            .get_ray(Direction::East);
    }

    attacks |= sq.get_ray(Direction::West);
    if sq.get_ray(Direction::West) & blockers != BitBoard::ZERO {
        attacks &= !(sq.get_ray(Direction::West) & blockers)
            .msb()
            .get_ray(Direction::West);
    }
    attacks
}

pub fn bishop_attacks_for_init(sq: SQ, blockers: BitBoard) -> BitBoard {
    let mut attacks: BitBoard = BitBoard::ZERO;
    attacks |= sq.get_ray(Direction::NorthWest);
    if sq.get_ray(Direction::NorthWest) & blockers != BitBoard::ZERO {
        attacks &= !(sq.get_ray(Direction::NorthWest) & blockers)
            .lsb()
            .get_ray(Direction::NorthWest);
    }

    attacks |= sq.get_ray(Direction::NorthEast);
    if sq.get_ray(Direction::NorthEast) & blockers != BitBoard::ZERO {
        attacks &= !(sq.get_ray(Direction::NorthEast) & blockers)
            .lsb()
            .get_ray(Direction::NorthEast);
    }

    attacks |= sq.get_ray(Direction::SouthEast);
    if sq.get_ray(Direction::SouthEast) & blockers != BitBoard::ZERO {
        attacks &= !(sq.get_ray(Direction::SouthEast) & blockers)
            .msb()
            .get_ray(Direction::SouthEast);
    }

    attacks |= sq.get_ray(Direction::SouthWest);
    if sq.get_ray(Direction::SouthWest) & blockers != BitBoard::ZERO {
        attacks &= !(sq.get_ray(Direction::SouthWest) & blockers)
            .msb()
            .get_ray(Direction::SouthWest);
    }
    attacks
}

pub fn init_pawn_attacks(pawn_attacks: &mut [[BitBoard; N_SQUARES]; N_COLORS]) {
    for sq in SQ::A1..=SQ::H8 {
        pawn_attacks[Color::White as usize][sq as usize] = WHITE_PAWN_ATTACKS[sq as usize];
        pawn_attacks[Color::Black as usize][sq as usize] = BLACK_PAWN_ATTACKS[sq as usize];
    }
}

pub fn init_attacks() {
    unsafe {
        init_pawn_attacks(&mut PAWN_ATTACKS);
    }
}
