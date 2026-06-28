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
    pub fn new(weights: &'static [[i16x16; D]; N]) -> Self {
        Self { weights }
    }
}

#[derive(Clone)]
struct Linear<const IN: usize, const OUT: usize> {
    weights: &'static [i16x16; IN],
    biases: &'static [i16; OUT],
}

impl<const IN: usize, const OUT: usize> Linear<IN, OUT> {
    pub fn new(weights: &'static [i16x16; IN], biases: &'static [i16; OUT]) -> Self {
        Self { weights, biases }
    }
}

#[derive(Clone)]
#[repr(C, align(64))]
struct Accumulator {
    acc: ColorMap<[i16x16; Network::L1 / Network::LANES]>,
    pop_count: i16,
}

#[derive(Clone)]
pub struct Network {
    input_layer: Embedding<{ Self::N_INPUTS }, { Self::L1 / Self::LANES }>,
    hidden_layers: [Linear<{ 2 * Self::L1 / Self::LANES }, 1>; Self::N_BUCKETS],

    stack: Vec<Accumulator>,
    idx: usize,
}

impl Network {
    pub fn new() -> Self {
        let bytes: &'static [u8] = &NETWORK.0;
        let (weights, biases) = bytes.split_at(2 * Self::W_I16);
        let mut w = Cursor { bytes: weights };
        let mut b = Cursor { bytes: biases };

        let input_layer = Embedding::new(w.take());
        let input_bias = *w.take::<[i16x16; Self::L1 / Self::LANES]>();

        // Weights and biases live in separate regions, so walk them in parallel.
        let hidden_layers = core::array::from_fn(|_| Linear::new(w.take(), b.take()));

        Self {
            input_layer,
            hidden_layers,
            stack: vec![
                Accumulator {
                    acc: ColorMap::new([input_bias; Color::N_COLORS]),
                    pop_count: 0
                };
                Self::N_ACCUMULATORS
            ],
            idx: 0,
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
            let pc_idx = pc.relative(color).index();
            let from_idx = pc_idx * SQ::N_SQUARES + from_sq.relative(color).index();
            let to_idx = pc_idx * SQ::N_SQUARES + to_sq.relative(color).index();

            let from_weights = self.input_layer.weights[from_idx].iter();
            let to_weights = self.input_layer.weights[to_idx].iter();

            cur.acc[color]
                .iter_mut()
                .zip(from_weights.zip(to_weights))
                .for_each(|(act, (&w_from, &w_to))| *act += w_to - w_from);
        }
    }

    fn update_activation<const SIGN: i16>(&mut self, pc: Piece, sq: SQ) {
        let cur = &mut self.stack[self.idx];

        for color in [Color::White, Color::Black] {
            let pc_idx = pc.relative(color).index();
            let sq_idx = sq.relative(color).index();
            let idx = pc_idx * SQ::N_SQUARES + sq_idx;

            cur.acc[color]
                .iter_mut()
                .zip(self.input_layer.weights[idx].iter())
                .for_each(|(act, &w)| *act += SIGN * w);
        }
        cur.pop_count += SIGN;
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

impl Network {
    const N_INPUTS: usize = Piece::N_PIECES * SQ::N_SQUARES;
    const N_ACCUMULATORS: usize = 1024;
    const L1: usize = 512;
    const N_BUCKETS: usize = 8;
    const BUCKET_DIV: usize = 32_usize.div_ceil(Self::N_BUCKETS);
    const LANES: usize = i16x16::LANES as usize;
    const NNUE2SCORE: i32 = 400;
    const INPUT_SCALE: i32 = 255;
    const HIDDEN_SCALE: i32 = 64;
    const COMB_SCALE: i32 = Self::HIDDEN_SCALE * Self::INPUT_SCALE;

    const W_I16: usize = Self::N_INPUTS * Self::L1 + Self::L1 + Self::N_BUCKETS * (2 * Self::L1);
    const B_I16: usize = Self::N_BUCKETS;
    const NET_BYTES: usize = 2 * Self::W_I16 + 2 * Self::B_I16;
}
