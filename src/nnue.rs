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
        self.deactivate(piece, from_sq);
        self.activate(piece, to_sq);
    }

    #[inline(always)]
    pub fn activate(&mut self, piece: Piece, sq: SQ) {
        let feature_idx =
            (piece.index() * SQ::N_SQUARES + sq.index()) * self.input_layer.activations.len();
        let weights = self.input_layer.weights
            [feature_idx..feature_idx + self.input_layer.activations.len()]
            .iter();

        for (activation, weight) in self.input_layer.activations.iter_mut().zip(weights) {
            *activation += weight;
        }
    }

    #[inline(always)]
    pub fn deactivate(&mut self, piece: Piece, sq: SQ) {
        let feature_idx =
            (piece.index() * SQ::N_SQUARES + sq.index()) * self.input_layer.activations.len();
        let weights = self.input_layer.weights
            [feature_idx..feature_idx + self.input_layer.activations.len()]
            .iter();

        for (activation, weight) in self.input_layer.activations.iter_mut().zip(weights) {
            *activation -= weight;
        }
    }

    pub fn eval(&self, board: &Board) -> Value {
        let bucket = (board.all_pieces().pop_count() as usize - 1) / 4;
        let bucket_idx = bucket * self.input_layer.activations.len();
        let mut output = self.hidden_layer.biases[bucket] as Value;

        let weights = self.hidden_layer.weights
            [bucket_idx..bucket_idx + self.input_layer.activations.len()]
            .iter();

        for (clipped_activation, weight) in self
            .input_layer
            .activations
            .iter()
            .map(|x| Self::clipped_relu(*x))
            .zip(weights)
        {
            output += clipped_activation * (*weight as Value);
        }
        output / (Self::SCALE * Self::SCALE)
    }

    #[inline(always)]
    fn clipped_relu(x: i16) -> Value {
        (x as Value).max(0).min(Self::SCALE)
    }
}

impl Network {
    const SCALE: i32 = 64;
}
