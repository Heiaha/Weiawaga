use super::bitboard::BitBoard;
use super::file::File;
use super::rank::Rank;
use super::square::{SQ, N_SQUARES};
use super::attacks;

const ROOK_MAGICS_INIT: [BitBoard; N_SQUARES] = [
    B!(0x0080001020400080), B!(0x0040001000200040), B!(0x0080081000200080), B!(0x0080040800100080),
    B!(0x0080020400080080), B!(0x0080010200040080), B!(0x0080008001000200), B!(0x0080002040800100),
    B!(0x0000800020400080), B!(0x0000400020005000), B!(0x0000801000200080), B!(0x0000800800100080),
    B!(0x0000800400080080), B!(0x0000800200040080), B!(0x0000800100020080), B!(0x0000800040800100),
    B!(0x0000208000400080), B!(0x0000404000201000), B!(0x0000808010002000), B!(0x0000808008001000),
    B!(0x0000808004000800), B!(0x0000808002000400), B!(0x0000010100020004), B!(0x0000020000408104),
    B!(0x0000208080004000), B!(0x0000200040005000), B!(0x0000100080200080), B!(0x0000080080100080),
    B!(0x0000040080080080), B!(0x0000020080040080), B!(0x0000010080800200), B!(0x0000800080004100),
    B!(0x0000204000800080), B!(0x0000200040401000), B!(0x0000100080802000), B!(0x0000080080801000),
    B!(0x0000040080800800), B!(0x0000020080800400), B!(0x0000020001010004), B!(0x0000800040800100),
    B!(0x0000204000808000), B!(0x0000200040008080), B!(0x0000100020008080), B!(0x0000080010008080),
    B!(0x0000040008008080), B!(0x0000020004008080), B!(0x0000010002008080), B!(0x0000004081020004),
    B!(0x0000204000800080), B!(0x0000200040008080), B!(0x0000100020008080), B!(0x0000080010008080),
    B!(0x0000040008008080), B!(0x0000020004008080), B!(0x0000800100020080), B!(0x0000800041000080),
    B!(0x00FFFCDDFCED714A), B!(0x007FFCDDFCED714A), B!(0x003FFFCDFFD88096), B!(0x0000040810002101),
    B!(0x0001000204080011), B!(0x0001000204000801), B!(0x0001000082000401), B!(0x0001FFFAABFAD1A2)
];

const BISHOP_MAGICS_INIT: [BitBoard; N_SQUARES] = [
    B!(0x0002020202020200), B!(0x0002020202020000), B!(0x0004010202000000), B!(0x0004040080000000),
    B!(0x0001104000000000), B!(0x0000821040000000), B!(0x0000410410400000), B!(0x0000104104104000),
    B!(0x0000040404040400), B!(0x0000020202020200), B!(0x0000040102020000), B!(0x0000040400800000),
    B!(0x0000011040000000), B!(0x0000008210400000), B!(0x0000004104104000), B!(0x0000002082082000),
    B!(0x0004000808080800), B!(0x0002000404040400), B!(0x0001000202020200), B!(0x0000800802004000),
    B!(0x0000800400A00000), B!(0x0000200100884000), B!(0x0000400082082000), B!(0x0000200041041000),
    B!(0x0002080010101000), B!(0x0001040008080800), B!(0x0000208004010400), B!(0x0000404004010200),
    B!(0x0000840000802000), B!(0x0000404002011000), B!(0x0000808001041000), B!(0x0000404000820800),
    B!(0x0001041000202000), B!(0x0000820800101000), B!(0x0000104400080800), B!(0x0000020080080080),
    B!(0x0000404040040100), B!(0x0000808100020100), B!(0x0001010100020800), B!(0x0000808080010400),
    B!(0x0000820820004000), B!(0x0000410410002000), B!(0x0000082088001000), B!(0x0000002011000800),
    B!(0x0000080100400400), B!(0x0001010101000200), B!(0x0002020202000400), B!(0x0001010101000200),
    B!(0x0000410410400000), B!(0x0000208208200000), B!(0x0000002084100000), B!(0x0000000020880000),
    B!(0x0000001002020000), B!(0x0000040408020000), B!(0x0004040404040000), B!(0x0002020202020000),
    B!(0x0000104104104000), B!(0x0000002082082000), B!(0x0000000020841000), B!(0x0000000000208800),
    B!(0x0000000010020200), B!(0x0000000404080200), B!(0x0000040404040400), B!(0x0002020202020200)
];

pub static mut ROOK_MAGICS: [RookMagic; N_SQUARES] = [RookMagic::new(); N_SQUARES];
pub static mut BISHOP_MAGICS: [BishopMagic; N_SQUARES] = [BishopMagic::new(); N_SQUARES];

#[derive(Clone, Copy, Debug)]
pub struct BishopMagic {
    pub shift: u32,
    pub magic: BitBoard,
    pub attack_masks: BitBoard,
    pub attacks: [BitBoard; 512],
}

#[derive(Clone, Copy, Debug)]
pub struct RookMagic {
    pub shift: u32,
    pub magic: BitBoard,
    pub attack_masks: BitBoard,
    pub attacks: [BitBoard; 4096],
}

impl BishopMagic {
    const fn new() -> Self {
        BishopMagic {
            shift: 0,
            magic: BitBoard::ZERO,
            attack_masks: BitBoard::ZERO,
            attacks: [BitBoard::ZERO; 512],
        }
    }
}

impl RookMagic {
    const fn new() -> Self {
        RookMagic {
            shift: 0,
            magic: BitBoard::ZERO,
            attack_masks: BitBoard::ZERO,
            attacks: [BitBoard::ZERO; 4096],
        }
    }
}

//////////////////////////////////////////////
// Inits
//////////////////////////////////////////////

fn initialize_rook_magics(magics: &mut [RookMagic; N_SQUARES]) {
    let mut edges: BitBoard;
    let mut subset: BitBoard;
    let mut index: BitBoard;
    for sq in SQ::A1..=SQ::H8 {
        let sq_index = sq.index();
        magics[sq_index].magic = ROOK_MAGICS_INIT[sq_index];
        edges = ((Rank::Rank1.bb() | Rank::Rank8.bb()) & !sq.rank().bb())
            | ((File::FileA.bb() | File::FileH.bb()) & !sq.file().bb());

        magics[sq_index].attack_masks = (sq.rank().bb() ^ sq.file().bb()) & !edges;
        magics[sq_index].shift = (64 - magics[sq_index].attack_masks.pop_count()) as u32;
        subset = BitBoard::ZERO;
        loop {
            index = subset;
            index *= ROOK_MAGICS_INIT[sq_index];
            index >>= magics[sq_index].shift;
            magics[sq_index].attacks[index.0 as usize] = attacks::rook_attacks_for_init(sq, subset);
            subset = (subset - magics[sq_index].attack_masks) & magics[sq_index].attack_masks;
            if subset == BitBoard::ZERO {
                break;
            }
        }
    }
}

fn initialize_bishop_magics(magics: &mut [BishopMagic; N_SQUARES]) {
    let mut edges: BitBoard;
    let mut subset: BitBoard;
    let mut index: BitBoard;
    for sq in SQ::A1..=SQ::H8 {
        let sq_index = sq.index();
        magics[sq_index].magic = BISHOP_MAGICS_INIT[sq_index];
        edges = ((Rank::Rank1.bb() | Rank::Rank8.bb()) & !sq.rank().bb())
            | ((File::FileA.bb() | File::FileH.bb()) & !sq.file().bb());

        magics[sq_index].attack_masks = (sq.diagonal().bb() ^ sq.antidiagonal().bb()) & !edges;
        magics[sq_index].shift = (64 - magics[sq_index].attack_masks.pop_count()) as u32;
        subset = BitBoard::ZERO;
        loop {
            index = subset;
            index *= BISHOP_MAGICS_INIT[sq_index];
            index >>= magics[sq_index].shift;
            magics[sq_index].attacks[index.0 as usize] = attacks::bishop_attacks_for_init(sq, subset);
            subset = (subset - magics[sq_index].attack_masks) & magics[sq_index].attack_masks;
            if subset == BitBoard::ZERO {
                break;
            }
        }
    }
}

pub fn init_magics() {
    unsafe {
        initialize_rook_magics(&mut ROOK_MAGICS);
        initialize_bishop_magics(&mut BISHOP_MAGICS);
    }
}
