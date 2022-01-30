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
            output_layer: Layer::new(&[], &[NNUE_OUTPUT_BIAS]),
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

    pub fn eval(&mut self) -> Value {
        let mut output = self.output_layer.biases[0] as i32;
        let mut relud;

        for i in 0..self.hidden_layer.len() {
            relud = Self::clipped_relu(self.hidden_layer.activations[i]) as i32;
            output += relud * self.hidden_layer.weights[i] as i32;
        }

        (output / Self::SCALE) as Value
    }

    #[inline(always)]
    pub fn clipped_relu(x: i16) -> i16 {
        x.max(0).min(Self::SCALE as i16)
    }
}

impl Network {
    const SCALE: i32 = 64 * 64;
}
