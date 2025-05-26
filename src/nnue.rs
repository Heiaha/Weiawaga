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
        assert_eq!(biases.len(), OUT);
        Self { weights, biases }
    }
}

#[derive(Clone)]
pub struct Network {
    input_layer: Embedding<{ Self::N_INPUTS }, { Self::L1 / Self::LANES }>,
    hidden_layers: [Linear<{ 2 * Self::L1 / Self::LANES }, 1>; Self::N_BUCKETS],
    accumulator: ColorMap<[i16x16; Self::L1 / Self::LANES]>,
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
            accumulator: ColorMap::new([INPUT_LAYER_BIAS; Color::N_COLORS]),
            pop_count: 0,
        }
    }

    pub fn activate(&mut self, pc: Piece, sq: SQ) {
        self.update_activation::<1>(pc, sq);
    }

    pub fn deactivate(&mut self, pc: Piece, sq: SQ) {
        self.update_activation::<-1>(pc, sq);
    }

    pub fn move_piece(&mut self, pc: Piece, from_sq: SQ, to_sq: SQ) {
        for color in [Color::White, Color::Black] {
            let pc_idx = pc.relative(color).index();
            let from_sq_idx = from_sq.relative(color).index();
            let to_sq_idx = to_sq.relative(color).index();

            let from_idx = pc_idx * SQ::N_SQUARES + from_sq_idx;
            let to_idx = pc_idx * SQ::N_SQUARES + to_sq_idx;

            let from_weights = self.input_layer.weights[from_idx].iter();
            let to_weights = self.input_layer.weights[to_idx].iter();

            self.accumulator[color]
                .iter_mut()
                .zip(from_weights.zip(to_weights))
                .for_each(|(activation, (&w_from, &w_to))| *activation += w_to - w_from);
        }
    }

    fn update_activation<const SIGN: i16>(&mut self, pc: Piece, sq: SQ) {
        for color in [Color::White, Color::Black] {
            let pc_idx = pc.relative(color).index();
            let sq_idx = sq.relative(color).index();
            let idx = pc_idx * SQ::N_SQUARES + sq_idx;
            let weights = self.input_layer.weights[idx].iter();
            self.accumulator[color]
                .iter_mut()
                .zip(weights)
                .for_each(|(activation, &weight)| *activation += SIGN * weight);
        }
        self.pop_count += SIGN;
    }

    fn eval_color(&self, color: Color, weights: &[i16x16]) -> i32x8 {
        self.accumulator[color]
            .iter()
            .zip(weights)
            .map(|(&activation, &weight)| {
                let clamped = Self::clipped_relu(activation);
                (weight * clamped).dot(clamped)
            })
            .sum()
    }

    pub fn eval(&self, ctm: Color) -> i32 {
        let bucket = (self.pop_count as usize - 2) / Self::BUCKET_DIV;
        let hidden_layer = &self.hidden_layers[bucket];

        let output = self.eval_color(ctm, &hidden_layer.weights[..Self::L1 / Self::LANES])
            + self.eval_color(!ctm, &hidden_layer.weights[Self::L1 / Self::LANES..]);

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
    const L1: usize = 512;
    const N_BUCKETS: usize = 8;
    const BUCKET_DIV: usize = (32 + Self::N_BUCKETS - 1) / Self::N_BUCKETS;
    const LANES: usize = i16x16::LANES as usize;
    const NNUE2SCORE: i32 = 400;
    const INPUT_SCALE: i32 = 255;
    const HIDDEN_SCALE: i32 = 64;
    const COMB_SCALE: i32 = Self::HIDDEN_SCALE * Self::INPUT_SCALE;
}
