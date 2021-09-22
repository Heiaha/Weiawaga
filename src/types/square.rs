use bitboard::Bitboard;

pub struct SQ(pub u8);


impl Sq {
    pub const NO_SQ: SQ = SQ(64);

    pub fn to_bb(self) -> Bitboard {
        BitBoard(1) << self
    }

    pub fn rank(self) -> Rank {

    }

}