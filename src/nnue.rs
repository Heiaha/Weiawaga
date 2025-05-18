use super::nnue_weights::*;
use super::piece::*;
use super::square::*;
use super::types::*;

use wide::*;

#[derive(Clone)]
struct Embedding<const N: usize, const D: usize> {
    weights: &'static [[i16x16; D]; N],
    biases: &'static [i16x16; D],
}

impl<const N: usize, const D: usize> Embedding<N, D> {
    pub fn new(weights: &'static [[i16x16; D]; N], biases: &'static [i16x16; D]) -> Self {
        Self { weights, biases }
    }
}

#[derive(Clone)]
struct Linear<const IN: usize, const OUT: usize> {
    weights: &'static [i16x16],
    biases: &'static [i16],
}

impl<const IN: usize, const OUT: usize> Linear<IN, OUT> {
    pub fn new(weights: &'static [i16x16], biases: &'static [i16]) -> Self {
        assert_eq!(weights.len(), IN * OUT);
        Self { weights, biases }
    }
}

#[derive(Clone)]
pub struct Network {
    input_layer: Embedding<{ Self::N_INPUTS }, { Self::L1 / Self::LANES }>,
    hidden_layers: [Linear<{ 2 * Self::L1 / Self::LANES }, 1>; Self::N_BUCKETS],
    w_accumulator: [i16x16; Self::L1 / Self::LANES],
    b_accumulator: [i16x16; Self::L1 / Self::LANES],
    pop_count: i16,
}

impl Network {
    pub fn new() -> Self {
        Self {
            input_layer: Embedding::new(&INPUT_LAYER_WEIGHT, &INPUT_LAYER_BIAS),
            hidden_layers: [
                Linear::new(&HIDDEN_LAYER_0_WEIGHT, &HIDDEN_LAYER_0_BIAS),
                Linear::new(&HIDDEN_LAYER_1_WEIGHT, &HIDDEN_LAYER_1_BIAS),
                Linear::new(&HIDDEN_LAYER_2_WEIGHT, &HIDDEN_LAYER_2_BIAS),
                Linear::new(&HIDDEN_LAYER_3_WEIGHT, &HIDDEN_LAYER_3_BIAS),
                Linear::new(&HIDDEN_LAYER_4_WEIGHT, &HIDDEN_LAYER_4_BIAS),
                Linear::new(&HIDDEN_LAYER_5_WEIGHT, &HIDDEN_LAYER_5_BIAS),
                Linear::new(&HIDDEN_LAYER_6_WEIGHT, &HIDDEN_LAYER_6_BIAS),
                Linear::new(&HIDDEN_LAYER_7_WEIGHT, &HIDDEN_LAYER_7_BIAS),
            ],
            w_accumulator: INPUT_LAYER_BIAS,
            b_accumulator: INPUT_LAYER_BIAS,
            pop_count: 0,
        }
    }

    pub fn move_piece(&mut self, pc: Piece, from_sq: SQ, to_sq: SQ) {
        self.deactivate(pc, from_sq);
        self.activate(pc, to_sq);
    }

    pub fn activate(&mut self, pc: Piece, sq: SQ) {
        self.update_activation::<1>(pc, sq);
    }

    pub fn deactivate(&mut self, pc: Piece, sq: SQ) {
        self.update_activation::<-1>(pc, sq);
    }

    fn update_activation<const SIGN: i16>(&mut self, pc: Piece, sq: SQ) {
        let simd_sign = i16x16::splat(SIGN);

        let w_embedding_idx = pc.index() * SQ::N_SQUARES + sq.index();
        let b_embedding_idx = pc.flip().index() * SQ::N_SQUARES + sq.square_mirror().index();

        let w_weights = self.input_layer.weights[w_embedding_idx].iter();
        let b_weights = self.input_layer.weights[b_embedding_idx].iter();

        self.w_accumulator
            .iter_mut()
            .zip(w_weights)
            .for_each(|(activation, weight)| *activation += simd_sign * weight);

        self.b_accumulator
            .iter_mut()
            .zip(b_weights)
            .for_each(|(activation, weight)| *activation += simd_sign * weight);

        self.pop_count += SIGN;
    }

    pub fn eval(&self, ctm: Color) -> Value {
        let bucket = (self.pop_count as usize - 1) / Self::BUCKET_DIV;
        let hidden_layer = &self.hidden_layers[bucket];
        let (ctm_accumulator, nctm_accumulator) = match ctm {
            Color::White => (&self.w_accumulator, &self.b_accumulator),
            Color::Black => (&self.b_accumulator, &self.w_accumulator),
        };
        let mut output = i32x8::ZERO;

        output += ctm_accumulator
            .iter()
            .zip(&hidden_layer.weights[..Self::L1 / Self::LANES])
            .map(|(&activation, &weight)| {
                Self::clipped_relu(activation).dot(weight)
            })
            .sum::<i32x8>();

        output += nctm_accumulator
            .iter()
            .zip(&hidden_layer.weights[Self::L1 / Self::LANES..])
            .map(|(&activation, &weight)| {
                Self::clipped_relu(activation).dot(weight)
            })
            .sum::<i32x8>();

        Value::from(hidden_layer.biases[0]) * Self::NNUE2SCORE / Self::HIDDEN_SCALE
            + output.reduce_add() * Self::NNUE2SCORE / Self::COMB_SCALE
    }

    fn clipped_relu(x: i16x16) -> i16x16 {
        x.max(i16x16::ZERO)
            .min(i16x16::splat(Self::INPUT_SCALE as i16))
    }
}

impl Network {
    const N_INPUTS: usize = Piece::N_PIECES * SQ::N_SQUARES;
    const L1: usize = 256;
    const N_BUCKETS: usize = 8;
    const BUCKET_DIV: usize = 32 / Self::N_BUCKETS;
    const LANES: usize = i16x16::LANES as usize;
    const NNUE2SCORE: Value = 400;
    const INPUT_SCALE: Value = 255;
    const HIDDEN_SCALE: Value = 64;
    const COMB_SCALE: Value = Self::HIDDEN_SCALE * Self::INPUT_SCALE;
}
