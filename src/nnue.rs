use super::bitboard::*;
use super::piece::*;
use super::square::*;
use super::traits::*;
use super::types::*;

use wide::*;

#[repr(C, align(64))]
struct Aligned<const N: usize>([u8; N]);

static NETWORK: Aligned<{ Network::NET_BYTES }> = Aligned(*include_bytes!("network.bin"));

struct Cursor {
    bytes: &'static [u8],
}

impl Cursor {
    fn take<T: bytemuck::Pod>(&mut self) -> &'static T {
        let (head, tail) = self.bytes.split_at(core::mem::size_of::<T>());
        self.bytes = tail;
        bytemuck::from_bytes(head)
    }
}

#[derive(Clone)]
struct Embedding<const N: usize, const D: usize> {
    weights: &'static [[i16x16; D]; N],
}

impl<const N: usize, const D: usize> Embedding<N, D> {
    fn new(weights: &'static [[i16x16; D]; N]) -> Self {
        Self { weights }
    }
}

#[derive(Clone)]
struct Linear<const IN: usize, const OUT: usize> {
    weights: &'static [i16x16; IN],
    biases: &'static [i16; OUT],
}

impl<const IN: usize, const OUT: usize> Linear<IN, OUT> {
    fn new(weights: &'static [i16x16; IN], biases: &'static [i16; OUT]) -> Self {
        Self { weights, biases }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct FeatureCtx {
    bucket: usize,
    mirrored: bool,
}

impl FeatureCtx {
    fn new(ksq_rel: SQ) -> Self {
        let mirrored = ksq_rel.file() >= File::E;
        let ksq_norm = if mirrored { ksq_rel.hmirror() } else { ksq_rel };
        Self {
            bucket: Self::king_bucket(ksq_norm),
            mirrored,
        }
    }

    fn king_bucket(ksq_norm: SQ) -> usize {
        match ksq_norm.rank() {
            Rank::One => 0,
            Rank::Two => 1,
            Rank::Three | Rank::Four => 2,
            _ => 3,
        }
    }

    fn feature_idx(self, pc: Piece, sq: SQ, color: Color) -> usize {
        let sq_rel = sq.relative(color);
        let sq_rel = if self.mirrored {
            sq_rel.hmirror()
        } else {
            sq_rel
        };
        pc.relative(color).index() * SQ::N_SQUARES + sq_rel.index()
    }
}

#[derive(Clone)]
#[repr(C, align(64))]
struct Accumulator {
    acc: ColorMap<[i16x16; Network::L1 / Network::LANES]>,
    pop_count: i16,
    ctx: ColorMap<FeatureCtx>,
}

#[derive(Clone, Copy)]
#[repr(C, align(64))]
struct CacheEntry {
    acc: [i16x16; Network::L1 / Network::LANES],
    pieces: PieceMap<Bitboard>,
}

#[derive(Clone)]
pub struct Network {
    input_layers: [Embedding<{ Self::N_INPUTS }, { Self::L1 / Self::LANES }>; Self::N_KING_BUCKETS],
    hidden_layers: [Linear<{ 2 * Self::L1 / Self::LANES }, 1>; Self::N_BUCKETS],

    stack: Vec<Accumulator>,
    idx: usize,
    cache: ColorMap<[[CacheEntry; 2]; Self::N_KING_BUCKETS]>,
}

impl Network {
    const N_INPUTS: usize = Piece::N_PIECES * SQ::N_SQUARES;
    const N_KING_BUCKETS: usize = 4;
    const N_ACCUMULATORS: usize = 1024;
    const L1: usize = 512;
    const N_BUCKETS: usize = 8;
    const BUCKET_DIV: usize = 32_usize.div_ceil(Self::N_BUCKETS);
    const LANES: usize = i16x16::LANES as usize;
    const NNUE2SCORE: i32 = 400;
    const INPUT_SCALE: i32 = 255;
    const HIDDEN_SCALE: i32 = 64;
    const COMB_SCALE: i32 = Self::HIDDEN_SCALE * Self::INPUT_SCALE;

    const W_I16: usize = Self::N_KING_BUCKETS * Self::N_INPUTS * Self::L1
        + Self::L1
        + Self::N_BUCKETS * (2 * Self::L1);
    const B_I16: usize = Self::N_BUCKETS;
    const NET_BYTES: usize = 2 * Self::W_I16 + 2 * Self::B_I16;

    pub fn new() -> Self {
        let bytes: &'static [u8] = &NETWORK.0;
        let (weights, biases) = bytes.split_at(2 * Self::W_I16);
        let mut w = Cursor { bytes: weights };
        let mut b = Cursor { bytes: biases };

        let input_weights: [&'static [[i16x16; Self::L1 / Self::LANES]; Self::N_INPUTS];
            Self::N_KING_BUCKETS] = core::array::from_fn(|_| w.take());
        let input_bias = w.take::<[i16x16; Self::L1 / Self::LANES]>();
        let input_layers = input_weights.map(Embedding::new);

        let hidden_layers = core::array::from_fn(|_| Linear::new(w.take(), b.take()));

        let cold_entry = CacheEntry {
            acc: *input_bias,
            pieces: PieceMap::new([Bitboard::ZERO; Piece::N_PIECES]),
        };

        Self {
            input_layers,
            hidden_layers,
            stack: vec![
                Accumulator {
                    acc: ColorMap::new([*input_bias; Color::N_COLORS]),
                    pop_count: 0,
                    ctx: ColorMap::new([FeatureCtx::default(); Color::N_COLORS]),
                };
                Self::N_ACCUMULATORS
            ],
            idx: 0,
            cache: ColorMap::new([[[cold_entry; 2]; Self::N_KING_BUCKETS]; Color::N_COLORS]),
        }
    }

    #[inline]
    pub fn push(&mut self) {
        debug_assert!(self.idx < Self::N_ACCUMULATORS);
        let next = self.idx + 1;
        self.stack[next] = self.stack[self.idx].clone();
        self.idx = next;
    }

    #[inline]
    pub fn pop(&mut self) {
        debug_assert!(self.idx > 0);
        self.idx -= 1;
    }

    pub fn activate(&mut self, pc: Piece, sq: SQ) {
        self.update_activation::<1>(pc, sq);
    }

    pub fn deactivate(&mut self, pc: Piece, sq: SQ) {
        self.update_activation::<-1>(pc, sq);
    }

    pub fn move_piece_quiet(&mut self, pc: Piece, from_sq: SQ, to_sq: SQ) {
        let cur = &mut self.stack[self.idx];
        for color in [Color::White, Color::Black] {
            let ctx = cur.ctx[color];
            let layer = &self.input_layers[ctx.bucket];
            let from_idx = ctx.feature_idx(pc, from_sq, color);
            let to_idx = ctx.feature_idx(pc, to_sq, color);

            let from_weights = layer.weights[from_idx].iter();
            let to_weights = layer.weights[to_idx].iter();

            cur.acc[color]
                .iter_mut()
                .zip(from_weights.zip(to_weights))
                .for_each(|(act, (&w_from, &w_to))| *act += w_to - w_from);
        }
    }

    fn update_activation<const SIGN: i16>(&mut self, pc: Piece, sq: SQ) {
        let cur = &mut self.stack[self.idx];

        for color in [Color::White, Color::Black] {
            let ctx = cur.ctx[color];
            let idx = ctx.feature_idx(pc, sq, color);

            cur.acc[color]
                .iter_mut()
                .zip(self.input_layers[ctx.bucket].weights[idx].iter())
                .for_each(|(act, &w)| *act += SIGN * w);
        }
        cur.pop_count += SIGN;
    }

    pub fn needs_refresh(&self, color: Color, ksq_rel: SQ) -> bool {
        self.stack[self.idx].ctx[color] != FeatureCtx::new(ksq_rel)
    }

    pub fn refresh(&mut self, color: Color, ksq_rel: SQ, pieces: &PieceMap<Bitboard>) {
        let ctx = FeatureCtx::new(ksq_rel);
        let layer = &self.input_layers[ctx.bucket];
        let entry = &mut self.cache[color][ctx.bucket][ctx.mirrored as usize];

        for pc in Piece::iter(Piece::WhitePawn, Piece::BlackKing) {
            for sq in pieces[pc] & !entry.pieces[pc] {
                let idx = ctx.feature_idx(pc, sq, color);
                entry
                    .acc
                    .iter_mut()
                    .zip(layer.weights[idx].iter())
                    .for_each(|(act, &w)| *act += w);
            }
            for sq in entry.pieces[pc] & !pieces[pc] {
                let idx = ctx.feature_idx(pc, sq, color);
                entry
                    .acc
                    .iter_mut()
                    .zip(layer.weights[idx].iter())
                    .for_each(|(act, &w)| *act -= w);
            }
            entry.pieces[pc] = pieces[pc];
        }

        let cur = &mut self.stack[self.idx];
        cur.ctx[color] = ctx;
        cur.acc[color] = entry.acc;
    }

    pub fn eval(&self, ctm: Color) -> i32 {
        let acc = &self.stack[self.idx];
        let bucket = (acc.pop_count as usize - 2) / Self::BUCKET_DIV;
        let hidden_layer = &self.hidden_layers[bucket];

        let eval_color = |color, weights: &[i16x16]| -> i32x8 {
            acc.acc[color]
                .iter()
                .zip(weights)
                .map(|(&act, &w)| {
                    let clamped = Self::clipped_relu(act);
                    (w * clamped).dot(clamped)
                })
                .sum()
        };

        let output = eval_color(ctm, &hidden_layer.weights[..Self::L1 / Self::LANES])
            + eval_color(!ctm, &hidden_layer.weights[Self::L1 / Self::LANES..]);

        i32::from(hidden_layer.biases[0]) * Self::NNUE2SCORE / Self::HIDDEN_SCALE
            + (output.reduce_add() / Self::INPUT_SCALE) * Self::NNUE2SCORE / Self::COMB_SCALE
    }

    fn clipped_relu(x: i16x16) -> i16x16 {
        x.max(i16x16::ZERO)
            .min(i16x16::splat(Self::INPUT_SCALE as i16))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_idx(pc: Piece, sq: SQ, color: Color, ctx: FeatureCtx) -> usize {
        ctx.bucket * Network::N_INPUTS + ctx.feature_idx(pc, sq, color)
    }

    // Position "6k1/8/8/8/8/8/8/1K6 w - - 0 1": white king b1 (files a-d,
    // not mirrored), black king g8 (mirrored to b8 -> relative b1); both
    // perspectives in bucket 0.
    #[test]
    fn feature_indices_match_trainer_convention() {
        let wk = Piece::make_piece(Color::White, PieceType::King);
        let bk = Piece::make_piece(Color::Black, PieceType::King);

        let w_ctx = FeatureCtx::new(SQ::B1);
        let b_ctx = FeatureCtx::new(SQ::G8.relative(Color::Black));

        // White perspective.
        assert_eq!(flat_idx(wk, SQ::B1, Color::White, w_ctx), 321);
        assert_eq!(flat_idx(bk, SQ::G8, Color::White, w_ctx), 766);

        // Black perspective: piece colors flip, ranks flip, and the black
        // king's file mirrors the whole board.
        assert_eq!(flat_idx(wk, SQ::B1, Color::Black, b_ctx), 766);
        assert_eq!(flat_idx(bk, SQ::G8, Color::Black, b_ctx), 321);
    }

    // Position "6k1/8/8/3K4/8/8/8/8 w - - 0 1": white king d5 (rank 5,
    // bucket 3), black king g8 (mirrored to b1, bucket 0). Exercises the
    // up-board buckets for the white perspective only.
    #[test]
    fn feature_indices_bucket_offsets_match_trainer_convention() {
        let wk = Piece::make_piece(Color::White, PieceType::King);
        let bk = Piece::make_piece(Color::Black, PieceType::King);

        let w_ctx = FeatureCtx::new(SQ::D5);
        let b_ctx = FeatureCtx::new(SQ::G8.relative(Color::Black));

        assert_eq!(w_ctx.bucket, 3);
        assert_eq!(b_ctx.bucket, 0);

        // White perspective: every feature comes from bucket 3.
        assert_eq!(flat_idx(bk, SQ::G8, Color::White, w_ctx), 3070);
        assert_eq!(flat_idx(wk, SQ::D5, Color::White, w_ctx), 2659);

        // Black perspective: bucket 0, mirrored.
        assert_eq!(flat_idx(bk, SQ::G8, Color::Black, b_ctx), 321);
        assert_eq!(flat_idx(wk, SQ::D5, Color::Black, b_ctx), 732);
    }
}
