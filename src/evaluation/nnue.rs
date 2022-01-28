use crate::evaluation::nnue_weights::*;
use crate::evaluation::score::Value;
use crate::types::piece::Piece;
use crate::types::square::SQ;

const HIDDEN_LAYER_SIZE: usize = 128;
const SCALE: i32 = 64;

#[derive(Copy, Clone)]
pub struct Network {
    pub feature_weights: &'static [i16],
    pub hidden_weights: &'static [i16],
    pub hidden_activations: [i16; HIDDEN_LAYER_SIZE],
    pub output_bias: i32,
}

impl Network {
    #[inline(always)]
    pub fn set_piece_at(&mut self, piece: Piece, sq: SQ) {
        let feature_idx = piece.nn_index() * 64 + sq.index();
        for j in 0..HIDDEN_LAYER_SIZE {
            self.hidden_activations[j] += self.feature_weights[feature_idx * HIDDEN_LAYER_SIZE + j];
        }
    }

    #[inline(always)]
    pub fn remove_piece_at(&mut self, piece: Piece, sq: SQ) {
        let feature_idx = piece.nn_index() * 64 + sq.index();
        for j in 0..HIDDEN_LAYER_SIZE {
            self.hidden_activations[j] -= self.feature_weights[feature_idx * HIDDEN_LAYER_SIZE + j];
        }
    }

    pub fn eval(&mut self) -> Value {
        let mut output = self.output_bias;
        let mut relud;

        for i in 0..HIDDEN_LAYER_SIZE {
            relud = Self::clipped_relu(self.hidden_activations[i]) as i32;
            output += relud * self.hidden_weights[i] as i32;
        }
        (output/(SCALE * SCALE)) as Value
    }

    #[inline(always)]
    pub fn clipped_relu(x: i16) -> i16 {
        x.max(0).min(SCALE as i16)
    }
}

impl Default for Network {
    fn default() -> Self {
        Self {
            feature_weights: &NNUE_FEATURE_WEIGHTS,
            hidden_weights: &NNUE_HIDDEN_WEIGHTS,
            hidden_activations: NNUE_HIDDEN_BIASES,
            output_bias: NNUE_OUTPUT_BIAS,
        }
    }
}
