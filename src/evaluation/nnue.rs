use crate::evaluation::nnue_weights::*;
use crate::evaluation::score::Value;
use crate::types::piece::Piece;
use crate::types::square::SQ;

#[derive(Clone)]
struct Layer {
    weights: &'static [i16],
    biases: &'static [i16],
    activations: Vec<i16>, // used for incremental layer
}

impl Layer {
    pub fn new(weights: &'static [i16], biases: &'static [i16]) -> Self {
        Self {
            weights,
            biases,
            activations: Vec::from(biases),
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.activations.len()
    }
}

#[derive(Clone)]
pub struct Network {
    input_layer: Layer,
    hidden_layer: Layer,
    output_layer: Layer,
}

impl Network {
    pub fn new() -> Self {
        Self {
            input_layer: Layer::new(&NNUE_FEATURE_WEIGHTS, &[]),
            hidden_layer: Layer::new(&NNUE_HIDDEN_WEIGHTS, &NNUE_HIDDEN_BIASES),
            output_layer: Layer::new(&[], &NNUE_OUTPUT_BIASES),
        }
    }

    #[inline(always)]
    pub fn activate(&mut self, piece: Piece, sq: SQ) {
        let feature_idx = (piece.nn_index() * SQ::N_SQUARES + sq.index()) * self.hidden_layer.len();
        for j in 0..self.hidden_layer.len() {
            self.hidden_layer.activations[j] += self.input_layer.weights[feature_idx + j];
        }
    }

    #[inline(always)]
    pub fn deactivate(&mut self, piece: Piece, sq: SQ) {
        let feature_idx = (piece.nn_index() * SQ::N_SQUARES + sq.index()) * self.hidden_layer.len();
        for j in 0..self.hidden_layer.len() {
            self.hidden_layer.activations[j] -= self.input_layer.weights[feature_idx + j];
        }
    }

    pub fn eval(&self, bucket: usize) -> Value {
        let mut output = self.output_layer.biases[bucket] as Value;
        let bucket_idx = bucket * self.hidden_layer.len();

        for j in 0..self.hidden_layer.len() {
            output += Self::clipped_relu(self.hidden_layer.activations[j])
                * self.hidden_layer.weights[bucket_idx + j] as Value;
        }

        output / (Self::SCALE * Self::SCALE)
    }

    #[inline(always)]
    pub fn clipped_relu(x: i16) -> Value {
        (x as Value).max(0).min(Self::SCALE)
    }
}

impl Network {
    const SCALE: i32 = 64;
}
