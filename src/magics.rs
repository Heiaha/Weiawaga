use super::attacks::*;
use super::bitboard::*;
use super::square::*;
use crate::types::*;
use std::sync::LazyLock;

// Fancy magic bitboard implementation inspired by Rustfish's port of Stockfish

#[rustfmt::skip]
const BISHOP_MAGICS_INIT: SQMap<Bitboard> = SQMap::new([
    B!(0x007fbfbfbfbfbfff), B!(0x0000a060401007fc), B!(0x0001004008020000), B!(0x0000806004000000),
    B!(0x0000100400000000), B!(0x000021c100b20000), B!(0x0000040041008000), B!(0x00000fb0203fff80),
    B!(0x0000040100401004), B!(0x0000020080200802), B!(0x0000004010202000), B!(0x0000008060040000),
    B!(0x0000004402000000), B!(0x0000000801008000), B!(0x000007efe0bfff80), B!(0x0000000820820020),
    B!(0x0000400080808080), B!(0x00021f0100400808), B!(0x00018000c06f3fff), B!(0x0000258200801000),
    B!(0x0000240080840000), B!(0x000018000c03fff8), B!(0x00000a5840208020), B!(0x0000020008208020),
    B!(0x0000804000810100), B!(0x0001011900802008), B!(0x0000804000810100), B!(0x000100403c0403ff),
    B!(0x00078402a8802000), B!(0x0000101000804400), B!(0x0000080800104100), B!(0x00004004c0082008),
    B!(0x0001010120008020), B!(0x000080809a004010), B!(0x0007fefe08810010), B!(0x0003ff0f833fc080),
    B!(0x007fe08019003042), B!(0x003fffefea003000), B!(0x0000101010002080), B!(0x0000802005080804),
    B!(0x0000808080a80040), B!(0x0000104100200040), B!(0x0003ffdf7f833fc0), B!(0x0000008840450020),
    B!(0x00007ffc80180030), B!(0x007fffdd80140028), B!(0x00020080200a0004), B!(0x0000101010100020),
    B!(0x0007ffdfc1805000), B!(0x0003ffefe0c02200), B!(0x0000000820806000), B!(0x0000000008403000),
    B!(0x0000000100202000), B!(0x0000004040802000), B!(0x0004010040100400), B!(0x00006020601803f4),
    B!(0x0003ffdfdfc28048), B!(0x0000000820820020), B!(0x0000000008208060), B!(0x0000000000808020),
    B!(0x0000000001002020), B!(0x0000000401002008), B!(0x0000004040404040), B!(0x007fff9fdf7ff813),
]);

#[rustfmt::skip]
const ROOK_MAGICS_INIT: SQMap<Bitboard> = SQMap::new([
    B!(0x00280077ffebfffe), B!(0x2004010201097fff), B!(0x0010020010053fff), B!(0x0040040008004002),
    B!(0x7fd00441ffffd003), B!(0x4020008887dffffe), B!(0x004000888847ffff), B!(0x006800fbff75fffd),
    B!(0x000028010113ffff), B!(0x0020040201fcffff), B!(0x007fe80042ffffe8), B!(0x00001800217fffe8),
    B!(0x00001800073fffe8), B!(0x00001800e05fffe8), B!(0x00001800602fffe8), B!(0x000030002fffffa0),
    B!(0x00300018010bffff), B!(0x0003000c0085fffb), B!(0x0004000802010008), B!(0x0004002020020004),
    B!(0x0001002002002001), B!(0x0001001000801040), B!(0x0000004040008001), B!(0x0000006800cdfff4),
    B!(0x0040200010080010), B!(0x0000080010040010), B!(0x0004010008020008), B!(0x0000040020200200),
    B!(0x0002008010100100), B!(0x0000008020010020), B!(0x0000008020200040), B!(0x0000820020004020),
    B!(0x00fffd1800300030), B!(0x007fff7fbfd40020), B!(0x003fffbd00180018), B!(0x001fffde80180018),
    B!(0x000fffe0bfe80018), B!(0x0001000080202001), B!(0x0003fffbff980180), B!(0x0001fffdff9000e0),
    B!(0x00fffefeebffd800), B!(0x007ffff7ffc01400), B!(0x003fffbfe4ffe800), B!(0x001ffff01fc03000),
    B!(0x000fffe7f8bfe800), B!(0x0007ffdfdf3ff808), B!(0x0003fff85fffa804), B!(0x0001fffd75ffa802),
    B!(0x00ffffd7ffebffd8), B!(0x007fff75ff7fbfd8), B!(0x003fff863fbf7fd8), B!(0x001fffbfdfd7ffd8),
    B!(0x000ffff810280028), B!(0x0007ffd7f7feffd8), B!(0x0003fffc0c480048), B!(0x0001ffffafd7ffd8),
    B!(0x00ffffe4ffdfa3ba), B!(0x007fffef7ff3d3da), B!(0x003fffbfdfeff7fa), B!(0x001fffeff7fbfc22),
    B!(0x0000020408001001), B!(0x0007fffeffff77fd), B!(0x0003ffffbf7dfeec), B!(0x0001ffff9dffa333),
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
    magic_init: &SQMap<Bitboard>,
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
        magics.magics[sq] = magic_init[sq];

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
