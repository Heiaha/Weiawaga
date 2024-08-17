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
            input_layer: Layer::new(&INPUT_LAYER_WEIGHT, &INPUT_LAYER_BIAS),
            hidden_layer: Layer::new(&L1_WEIGHT, &L1_BIAS),
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
        let feature_idx =
            (piece.index() * SQ::N_SQUARES + sq.index()) * self.input_layer.activations.len();
        let weights = self.input_layer.weights
            [feature_idx..feature_idx + self.input_layer.activations.len()]
            .iter();

        self.input_layer
            .activations
            .iter_mut()
            .zip(weights)
            .for_each(|(activation, weight)| update_fn(activation, weight));
    }

    pub fn eval(&self) -> Value {
        let output = self.hidden_layer.biases[0] as Value
            + self
                .input_layer
                .activations
                .iter()
                .zip(self.hidden_layer.weights)
                .map(|(activation, weight)| {
                    (Self::clipped_relu(*activation) as Value) * (*weight as Value)
                })
                .sum::<Value>();

        Self::NNUE2SCORE * output / (Self::SCALE * Self::SCALE) as Value
    }

    fn clipped_relu(x: i16) -> i16 {
        x.clamp(0, Self::SCALE)
    }
}

impl Network {
    const SCALE: i16 = 64;
    const NNUE2SCORE: Value = 600;
}
