use super::bitboard::*;
use super::piece::*;
use super::square::*;
use super::traits::*;

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
    weights: &'static [[i16x32; D]; N],
}

impl<const N: usize, const D: usize> Embedding<N, D> {
    const fn new(weights: &'static [[i16x32; D]; N]) -> Self {
        Self { weights }
    }

    fn update<const SIGN: i16>(&self, idx: usize, acc: &mut [i16x32; D]) {
        acc.iter_mut()
            .zip(self.weights[idx].iter())
            .for_each(|(act, &w)| *act += SIGN * w);
    }

    fn add_sub(&self, add_idx: usize, sub_idx: usize, acc: &mut [i16x32; D]) {
        acc.iter_mut()
            .zip(
                self.weights[add_idx]
                    .iter()
                    .zip(self.weights[sub_idx].iter()),
            )
            .for_each(|(act, (&w_add, &w_sub))| *act += w_add - w_sub);
    }
}

#[derive(Clone)]
struct Linear<const IN: usize, const OUT: usize> {
    weights: &'static [i16x32; IN],
    biases: &'static [i16; OUT],
}

impl<const IN: usize, const OUT: usize> Linear<IN, OUT> {
    const fn new(weights: &'static [i16x32; IN], biases: &'static [i16; OUT]) -> Self {
        Self { weights, biases }
    }

    // Fused clipped-relu-square dot product over both perspective halves,
    // side to move first. Returns the raw quantized sum; scaling and the
    // bias are the caller's concern.
    fn forward(&self, stm: &[i16x32], nstm: &[i16x32]) -> i32 {
        let (stm_weights, nstm_weights) = self.weights.split_at(stm.len());

        let dot = |acts: &[i16x32], weights: &[i16x32]| -> i32x16 {
            acts.iter()
                .zip(weights)
                .map(|(&act, &w)| {
                    let clamped = Network::clipped_relu(act);
                    (w * clamped).dot(clamped)
                })
                .sum()
        };

        (dot(stm, stm_weights) + dot(nstm, nstm_weights)).reduce_add()
    }
}

#[derive(Clone)]
struct WdlLayer<const IN: usize> {
    weights: &'static [[f32; IN]; 3],
    biases: &'static [f32; 3],
}

impl<const IN: usize> WdlLayer<IN> {
    const fn new(weights: &'static [[f32; IN]; 3], biases: &'static [f32; 3]) -> Self {
        Self { weights, biases }
    }

    // Softmaxed loss/draw/win probabilities for the given activations.
    fn forward(&self, activations: &[f32; IN]) -> [f32; 3] {
        let mut logits = *self.biases;
        for (logit, weights) in logits.iter_mut().zip(self.weights) {
            *logit += weights
                .iter()
                .zip(activations)
                .map(|(w, a)| w * a)
                .sum::<f32>();
        }

        let max = logits.into_iter().fold(f32::MIN, f32::max);
        let exps = logits.map(|logit| (logit - max).exp());
        let sum = exps.iter().sum::<f32>();
        exps.map(|exp| exp / sum)
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
        pc.relative(color).index() * SQ::COUNT + sq_rel.index()
    }
}

#[derive(Clone)]
#[repr(C, align(64))]
struct Accumulator {
    acc: ColorMap<[i16x32; Network::L1 / Network::LANES]>,
    pop_count: i16,
    ctx: ColorMap<FeatureCtx>,
}

#[derive(Clone, Copy)]
#[repr(C, align(64))]
struct CacheEntry {
    acc: [i16x32; Network::L1 / Network::LANES],
    pieces: PieceMap<Bitboard>,
}

#[derive(Clone)]
pub struct Network {
    input_layers: [Embedding<{ Self::N_INPUTS }, { Self::L1 / Self::LANES }>; Self::N_KING_BUCKETS],
    hidden_layers: [Linear<{ 2 * Self::L1 / Self::LANES }, 1>; Self::N_BUCKETS],
    wdl_layers: [WdlLayer<{ 2 * Self::L1 }>; Self::N_BUCKETS],

    stack: Vec<Accumulator>,
    idx: usize,
    cache: ColorMap<[[CacheEntry; 2]; Self::N_KING_BUCKETS]>,
}

impl Network {
    pub fn new() -> Self {
        let bytes: &'static [u8] = &NETWORK.0;
        let (weights, biases) = bytes.split_at(2 * Self::W_I16);
        let mut w = Cursor { bytes: weights };
        let mut b = Cursor { bytes: biases };

        let input_weights: [&'static [[i16x32; Self::L1 / Self::LANES]; Self::N_INPUTS];
            Self::N_KING_BUCKETS] = core::array::from_fn(|_| w.take());
        let input_bias = w.take::<[i16x32; Self::L1 / Self::LANES]>();
        let input_layers = input_weights.map(Embedding::new);

        let hidden_layers = core::array::from_fn(|_| Linear::new(w.take(), b.take()));

        let wdl_weights: [_; Self::N_BUCKETS] = core::array::from_fn(|_| b.take());
        let wdl_biases: [_; Self::N_BUCKETS] = core::array::from_fn(|_| b.take());
        let wdl_layers = core::array::from_fn(|i| WdlLayer::new(wdl_weights[i], wdl_biases[i]));

        let cold_entry = CacheEntry {
            acc: *input_bias,
            pieces: PieceMap::default(),
        };

        Self {
            input_layers,
            hidden_layers,
            wdl_layers,
            stack: vec![
                Accumulator {
                    acc: ColorMap::new([*input_bias; Color::COUNT]),
                    pop_count: 0,
                    ctx: ColorMap::default(),
                };
                Self::N_ACCUMULATORS
            ],
            idx: 0,
            cache: ColorMap::new([[[cold_entry; 2]; Self::N_KING_BUCKETS]; Color::COUNT]),
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
            let from_idx = ctx.feature_idx(pc, from_sq, color);
            let to_idx = ctx.feature_idx(pc, to_sq, color);

            self.input_layers[ctx.bucket].add_sub(to_idx, from_idx, &mut cur.acc[color]);
        }
    }

    fn update_activation<const SIGN: i16>(&mut self, pc: Piece, sq: SQ) {
        let cur = &mut self.stack[self.idx];

        for color in [Color::White, Color::Black] {
            let ctx = cur.ctx[color];
            let idx = ctx.feature_idx(pc, sq, color);

            self.input_layers[ctx.bucket].update::<SIGN>(idx, &mut cur.acc[color]);
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

        for pc in Piece::iter() {
            for sq in pieces[pc] & !entry.pieces[pc] {
                layer.update::<1>(ctx.feature_idx(pc, sq, color), &mut entry.acc);
            }
            for sq in entry.pieces[pc] & !pieces[pc] {
                layer.update::<-1>(ctx.feature_idx(pc, sq, color), &mut entry.acc);
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

        let output = hidden_layer.forward(&acc.acc[ctm], &acc.acc[!ctm]);

        i32::from(hidden_layer.biases[0]) * Self::NNUE2SCORE / Self::HIDDEN_SCALE
            + (output / Self::INPUT_SCALE) * Self::NNUE2SCORE / Self::COMB_SCALE
    }

    fn clipped_relu(x: i16x32) -> i16x32 {
        x.max(i16x32::ZERO)
            .min(i16x32::splat(Self::INPUT_SCALE as i16))
    }

    pub fn wdl(&self, ctm: Color) -> [f32; 3] {
        let acc = &self.stack[self.idx];

        let mut activations = [0.0; 2 * Self::L1];
        let lanes = [ctm, !ctm]
            .into_iter()
            .flat_map(|color| &acc.acc[color])
            .flat_map(|&act| Self::clipped_relu(act).to_array());
        for (activation, x) in activations.iter_mut().zip(lanes) {
            let a = f32::from(x) / Self::INPUT_SCALE as f32;
            *activation = a * a;
        }

        let bucket = (acc.pop_count as usize - 2) / Self::BUCKET_DIV;
        self.wdl_layers[bucket].forward(&activations)
    }
}

impl Network {
    const N_INPUTS: usize = Piece::COUNT * SQ::COUNT;
    const N_KING_BUCKETS: usize = 4;
    const N_ACCUMULATORS: usize = 1024;
    const L1: usize = 768;
    const N_BUCKETS: usize = 8;
    const BUCKET_DIV: usize = 32_usize.div_ceil(Self::N_BUCKETS);
    const LANES: usize = i16x32::LANES as usize;
    const NNUE2SCORE: i32 = 400;
    const INPUT_SCALE: i32 = 255;
    const HIDDEN_SCALE: i32 = 64;
    const COMB_SCALE: i32 = Self::HIDDEN_SCALE * Self::INPUT_SCALE;

    const W_I16: usize = Self::N_KING_BUCKETS * Self::N_INPUTS * Self::L1
        + Self::L1
        + Self::N_BUCKETS * (2 * Self::L1);
    const B_I16: usize = Self::N_BUCKETS;
    const WDL_F32: usize = Self::N_BUCKETS * (3 * (2 * Self::L1) + 3);
    const NET_BYTES: usize = 2 * Self::W_I16 + 2 * Self::B_I16 + 4 * Self::WDL_F32;
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

    #[test]
    fn clone_shares_weights_with_the_static_blob() {
        let a = Network::new();
        let b = a.clone();

        // Same pointers after clone, and they point into NETWORK itself.
        assert!(std::ptr::eq(
            a.input_layers[0].weights,
            b.input_layers[0].weights
        ));
        assert!(std::ptr::eq(
            a.hidden_layers[0].weights,
            b.hidden_layers[0].weights
        ));
        assert!(std::ptr::eq(
            a.wdl_layers[0].weights,
            b.wdl_layers[0].weights
        ));

        let blob = NETWORK.0.as_ptr_range();
        let w = a.input_layers[0].weights.as_ptr() as *const u8;
        assert!(blob.contains(&w));

        println!(
            "size_of::<Network>() = {} bytes (blob = {} bytes)",
            std::mem::size_of::<Network>(),
            Network::NET_BYTES,
        );
    }
}
