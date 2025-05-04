use super::nnue_weights::*;
use super::piece::*;
use super::square::*;
use super::types::*;

#[derive(Clone)]
struct Embedding<const N: usize, const D: usize> {
    weights: &'static [[i16; D]; N],
    biases: &'static [i16; D],
}

impl<const N: usize, const D: usize> Embedding<N, D> {
    pub fn new(weights: &'static [[i16; D]; N], biases: &'static [i16; D]) -> Self {
        Self { weights, biases }
    }
}

#[derive(Clone)]
struct Linear<const IN: usize, const OUT: usize> {
    weights: &'static [i16],
    biases: &'static [i16],
}

impl<const IN: usize, const OUT: usize> Linear<IN, OUT> {
    pub fn new(weights: &'static [i16], biases: &'static [i16]) -> Self {
        assert_eq!(weights.len(), IN * OUT);
        Self { weights, biases }
    }
}

#[derive(Clone)]
pub struct Network {
    input_layer: Embedding<{ Self::N_INPUTS }, { Self::L1 }>,
    hidden_layers: [Linear<{ Self::L1 }, 1>; Self::N_BUCKETS],
    accumulator: [i16; Self::L1],
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
            accumulator: INPUT_LAYER_BIAS,
            pop_count: 0,
        }
    }

    pub fn move_piece(&mut self, piece: Piece, from_sq: SQ, to_sq: SQ) {
        self.deactivate(piece, from_sq);
        self.activate(piece, to_sq);
    }

    pub fn activate(&mut self, piece: Piece, sq: SQ) {
        self.update_activation(piece, sq, |activation, weight| *activation += weight);
    }

    pub fn deactivate(&mut self, piece: Piece, sq: SQ) {
        self.update_activation(piece, sq, |activation, weight| *activation -= weight);
    }

    fn update_activation(
        &mut self,
        piece: Piece,
        sq: SQ,
        mut update_fn: impl FnMut(&mut i16, &i16),
    ) {
        let embedding_idx: usize = piece.index() * SQ::N_SQUARES + sq.index();
        let weights = self.input_layer.weights[embedding_idx].iter();

        self.accumulator
            .iter_mut()
            .zip(weights)
            .for_each(|(activation, weight)| update_fn(activation, weight));

        update_fn(&mut self.pop_count, &1);
    }

    pub fn eval(&self) -> Value {
        let bucket = (self.pop_count as usize - 1) / Self::BUCKET_DIV;

        let hidden_layer = &self.hidden_layers[bucket];
        let output = self
            .accumulator
            .iter()
            .zip(hidden_layer.weights)
            .map(|(&activation, &weight)| Self::clipped_relu(activation) * Value::from(weight))
            .sum::<Value>();

        Value::from(hidden_layer.biases[0]) * Self::NNUE2SCORE / Self::HIDDEN_SCALE
            + output * Self::NNUE2SCORE / Self::COMB_SCALE
    }

    fn clipped_relu(x: i16) -> Value {
        Value::from(x).clamp(0, Self::INPUT_SCALE)
    }
}

impl Network {
    const N_INPUTS: usize = Piece::N_PIECES * SQ::N_SQUARES;
    const L1: usize = 512;
    const N_BUCKETS: usize = 8;
    const BUCKET_DIV: usize = 32 / Self::N_BUCKETS;
    const NNUE2SCORE: Value = 400;
    const INPUT_SCALE: Value = 255;
    const HIDDEN_SCALE: Value = 64;
    const COMB_SCALE: Value = Self::HIDDEN_SCALE * Self::INPUT_SCALE;
}
