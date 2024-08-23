use super::nnue_weights::*;
use super::piece::*;
use super::square::*;
use super::types::*;

#[derive(Clone)]
struct Layer {
    weights: &'static [i16],
    biases: &'static [i16],
}

impl Layer {
    pub fn new(weights: &'static [i16], biases: &'static [i16]) -> Self {
        Self { weights, biases }
    }
}

#[derive(Clone)]
pub struct Network {
    input_layer: Layer,
    hidden_layer: Layer,
    accumulator: [i16; INPUT_LAYER_BIAS.len()],
}

impl Network {
    pub fn new() -> Self {
        Self {
            input_layer: Layer::new(&INPUT_LAYER_WEIGHT, &INPUT_LAYER_BIAS),
            hidden_layer: Layer::new(&HIDDEN_LAYER_WEIGHT, &HIDDEN_LAYER_BIAS),
            accumulator: INPUT_LAYER_BIAS,
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
            (piece.index() * SQ::N_SQUARES + sq.index()) * self.input_layer.biases.len();
        let weights = self.input_layer.weights
            [feature_idx..feature_idx + self.input_layer.biases.len()]
            .iter();

        self.accumulator
            .iter_mut()
            .zip(weights)
            .for_each(|(activation, weight)| update_fn(activation, weight));
    }

    pub fn eval(&self) -> Value {
        let output = self
            .accumulator
            .iter()
            .zip(self.hidden_layer.weights)
            .map(|(activation, weight)| (Self::clipped_relu(*activation)) * (*weight as Value))
            .sum::<Value>();

        (Value::from(self.hidden_layer.biases[0]) + output / Self::INPUT_SCALE) * Self::NNUE2SCORE
            / Self::HIDDEN_SCALE
    }

    fn clipped_relu(x: i16) -> Value {
        Value::from(x).clamp(0, Self::INPUT_SCALE)
    }
}

impl Network {
    const INPUT_SCALE: Value = 255;
    const HIDDEN_SCALE: Value = 64;
    const NNUE2SCORE: Value = 400;
}
