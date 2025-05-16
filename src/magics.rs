use super::attacks::*;
use super::bitboard::*;
use super::square::*;
use crate::types::*;
use std::sync::LazyLock;

// Fancy magic bitboard implementation inspired by Rustfish's port of Stockfish

struct MagicInit {
    magic: Bitboard,
    index: usize,
}

macro_rules! M {
    ($x:expr, $y:expr) => {
        MagicInit {
            magic: Bitboard($x),
            index: $y,
        }
    };
}

#[rustfmt::skip]
const BISHOP_MAGICS_INIT: SQMap<MagicInit> = SQMap::new([
    M!(0x007fbfbfbfbfbfff,  5378), M!(0x0000a060401007fc,  4093), M!(0x0001004008020000,  4314), M!(0x0000806004000000,  6587),
    M!(0x0000100400000000,  6491), M!(0x000021c100b20000,  6330), M!(0x0000040041008000,  5609), M!(0x00000fb0203fff80, 22236),
    M!(0x0000040100401004,  6106), M!(0x0000020080200802,  5625), M!(0x0000004010202000, 16785), M!(0x0000008060040000, 16817),
    M!(0x0000004402000000,  6842), M!(0x0000000801008000,  7003), M!(0x000007efe0bfff80,  4197), M!(0x0000000820820020,  7356),
    M!(0x0000400080808080,  4602), M!(0x00021f0100400808,  4538), M!(0x00018000c06f3fff, 29531), M!(0x0000258200801000, 45393),
    M!(0x0000240080840000, 12420), M!(0x000018000c03fff8, 15763), M!(0x00000a5840208020,  5050), M!(0x0000020008208020,  4346),
    M!(0x0000804000810100,  6074), M!(0x0001011900802008,  7866), M!(0x0000804000810100, 32139), M!(0x000100403c0403ff, 57673),
    M!(0x00078402a8802000, 55365), M!(0x0000101000804400, 15818), M!(0x0000080800104100,  5562), M!(0x00004004c0082008,  6390),
    M!(0x0001010120008020,  7930), M!(0x000080809a004010, 13329), M!(0x0007fefe08810010,  7170), M!(0x0003ff0f833fc080, 27267),
    M!(0x007fe08019003042, 53787), M!(0x003fffefea003000,  5097), M!(0x0000101010002080,  6643), M!(0x0000802005080804,  6138),
    M!(0x0000808080a80040,  7418), M!(0x0000104100200040,  7898), M!(0x0003ffdf7f833fc0, 42012), M!(0x0000008840450020, 57350),
    M!(0x00007ffc80180030, 22813), M!(0x007fffdd80140028, 56693), M!(0x00020080200a0004,  5818), M!(0x0000101010100020,  7098),
    M!(0x0007ffdfc1805000,  4451), M!(0x0003ffefe0c02200,  4709), M!(0x0000000820806000,  4794), M!(0x0000000008403000, 13364),
    M!(0x0000000100202000,  4570), M!(0x0000004040802000,  4282), M!(0x0004010040100400, 14964), M!(0x00006020601803f4,  4026),
    M!(0x0003ffdfdfc28048,  4826), M!(0x0000000820820020,  7354), M!(0x0000000008208060,  4848), M!(0x0000000000808020, 15946),
    M!(0x0000000001002020, 14932), M!(0x0000000401002008, 16588), M!(0x0000004040404040,  6905), M!(0x007fff9fdf7ff813, 16076),
]);

#[rustfmt::skip]
const ROOK_MAGICS_INIT: SQMap<MagicInit> = SQMap::new([
    M!(0x00280077ffebfffe, 26304), M!(0x2004010201097fff, 35520), M!(0x0010020010053fff, 38592), M!(0x0040040008004002,  8026),
    M!(0x7fd00441ffffd003, 22196), M!(0x4020008887dffffe, 80870), M!(0x004000888847ffff, 76747), M!(0x006800fbff75fffd, 30400),
    M!(0x000028010113ffff, 11115), M!(0x0020040201fcffff, 18205), M!(0x007fe80042ffffe8, 53577), M!(0x00001800217fffe8, 62724),
    M!(0x00001800073fffe8, 34282), M!(0x00001800e05fffe8, 29196), M!(0x00001800602fffe8, 23806), M!(0x000030002fffffa0, 49481),
    M!(0x00300018010bffff,  2410), M!(0x0003000c0085fffb, 36498), M!(0x0004000802010008, 24478), M!(0x0004002020020004, 10074),
    M!(0x0001002002002001, 79315), M!(0x0001001000801040, 51779), M!(0x0000004040008001, 13586), M!(0x0000006800cdfff4, 19323),
    M!(0x0040200010080010, 70612), M!(0x0000080010040010, 83652), M!(0x0004010008020008, 63110), M!(0x0000040020200200, 34496),
    M!(0x0002008010100100, 84966), M!(0x0000008020010020, 54341), M!(0x0000008020200040, 60421), M!(0x0000820020004020, 86402),
    M!(0x00fffd1800300030, 50245), M!(0x007fff7fbfd40020, 76622), M!(0x003fffbd00180018, 84676), M!(0x001fffde80180018, 78757),
    M!(0x000fffe0bfe80018, 37346), M!(0x0001000080202001,   370), M!(0x0003fffbff980180, 42182), M!(0x0001fffdff9000e0, 45385),
    M!(0x00fffefeebffd800, 61659), M!(0x007ffff7ffc01400, 12790), M!(0x003fffbfe4ffe800, 16762), M!(0x001ffff01fc03000,     0),
    M!(0x000fffe7f8bfe800, 38380), M!(0x0007ffdfdf3ff808, 11098), M!(0x0003fff85fffa804, 21803), M!(0x0001fffd75ffa802, 39189),
    M!(0x00ffffd7ffebffd8, 58628), M!(0x007fff75ff7fbfd8, 44116), M!(0x003fff863fbf7fd8, 78357), M!(0x001fffbfdfd7ffd8, 44481),
    M!(0x000ffff810280028, 64134), M!(0x0007ffd7f7feffd8, 41759), M!(0x0003fffc0c480048,  1394), M!(0x0001ffffafd7ffd8, 40910),
    M!(0x00ffffe4ffdfa3ba, 66516), M!(0x007fffef7ff3d3da,  3897), M!(0x003fffbfdfeff7fa,  3930), M!(0x001fffeff7fbfc22, 72934),
    M!(0x0000020408001001, 72662), M!(0x0007fffeffff77fd, 56325), M!(0x0003ffffbf7dfeec, 66501), M!(0x0001ffff9dffa333, 14826),
]);

pub struct Magics {
    masks: SQMap<Bitboard>,
    magics: SQMap<Bitboard>,
    pub attacks: SQMap<Vec<Bitboard>>,
    shift: u8,
}

impl Magics {
    pub fn index(&self, sq: SQ, occ: Bitboard) -> usize {
        (((occ & self.masks[sq]) * self.magics[sq]) >> self.shift).0 as usize
    }
}

pub static ROOK_MAGICS: LazyLock<Magics> =
    LazyLock::new(|| init_magics_type(&ROOK_MAGICS_INIT, rook_attacks_for_init, 64 - 12));

pub static BISHOP_MAGICS: LazyLock<Magics> =
    LazyLock::new(|| init_magics_type(&BISHOP_MAGICS_INIT, bishop_attacks_for_init, 64 - 9));

//////////////////////////////////////////////
// Inits
//////////////////////////////////////////////

fn init_magics_type(
    magic_init: &SQMap<MagicInit>,
    slow_attacks_gen: fn(SQ, Bitboard) -> Bitboard,
    shift: u8,
) -> Magics {
    let mut magics = Magics {
        masks: SQMap::new([Bitboard::ZERO; SQ::N_SQUARES]),
        magics: SQMap::new([Bitboard::ZERO; SQ::N_SQUARES]),
        attacks: SQMap::new([const { Vec::new() }; SQ::N_SQUARES]),
        shift,
    };

    for sq in Bitboard::ALL {
        let edges = ((Rank::One.bb() | Rank::Eight.bb()) & !sq.rank().bb())
            | ((File::A.bb() | File::H.bb()) & !sq.file().bb());
        magics.masks[sq] = slow_attacks_gen(sq, Bitboard::ZERO) & !edges;
        magics.magics[sq] = magic_init[sq].magic;

        let mut subset = Bitboard::ZERO;
        let mut entries = Vec::new();
        let mut max_index = 0;

        loop {
            let idx = magics.index(sq, subset);
            let attack = slow_attacks_gen(sq, subset);
            entries.push((idx, attack));
            max_index = max_index.max(idx);

            subset = (subset - magics.masks[sq]) & magics.masks[sq];
            if subset == Bitboard::ZERO {
                break;
            }
        }

        let mut table = vec![Bitboard::ZERO; max_index + 1];
        for (idx, att) in entries {
            table[idx] = att;
        }
        magics.attacks[sq] = table;
    }
    magics
}
