use super::board::*;
use super::nnue_weights::*;
use super::piece::*;
use super::square::*;
use super::types::*;

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
}

#[derive(Clone)]
pub struct Network {
    input_layer: Layer,
    hidden_layer: Layer,
}

impl Network {
    pub fn new() -> Self {
        Self {
            input_layer: Layer::new(&NNUE_INPUT_WEIGHTS, &NNUE_INPUT_BIASES),
            hidden_layer: Layer::new(&NNUE_HIDDEN_WEIGHTS, &NNUE_HIDDEN_BIASES),
        }
    }

    #[inline(always)]
    pub fn move_piece(&mut self, piece: Piece, from_sq: SQ, to_sq: SQ) {
        let pc_idx = piece.index() * SQ::N_SQUARES;
        let from_idx = (pc_idx + from_sq.index()) * self.input_layer.activations.len();
        let to_idx = (pc_idx + to_sq.index()) * self.input_layer.activations.len();

        for j in 0..self.input_layer.activations.len() {
            self.input_layer.activations[j] -= self.input_layer.weights[from_idx + j];
            self.input_layer.activations[j] += self.input_layer.weights[to_idx + j];
        }
    }

    #[inline(always)]
    pub fn activate(&mut self, piece: Piece, sq: SQ) {
        let feature_idx =
            (piece.index() * SQ::N_SQUARES + sq.index()) * self.input_layer.activations.len();
        for j in 0..self.input_layer.activations.len() {
            self.input_layer.activations[j] += self.input_layer.weights[feature_idx + j];
        }
    }

    #[inline(always)]
    pub fn deactivate(&mut self, piece: Piece, sq: SQ) {
        let feature_idx =
            (piece.index() * SQ::N_SQUARES + sq.index()) * self.input_layer.activations.len();
        for j in 0..self.input_layer.activations.len() {
            self.input_layer.activations[j] -= self.input_layer.weights[feature_idx + j];
        }
    }

    pub fn eval(&self, board: &Board) -> Value {
        let bucket = (board.all_pieces().pop_count() as usize - 1) / 4;
        let bucket_idx = bucket * self.input_layer.activations.len();
        let mut output = self.hidden_layer.biases[bucket] as Value;

        for j in 0..self.input_layer.activations.len() {
            output += Self::clipped_relu(self.input_layer.activations[j])
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
