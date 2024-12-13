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
    hidden_layers: [Layer; 8],
    psqt_layer: Layer,
    psqt_value: i16,
    accumulator: [i16; INPUT_LAYER_BIAS.len()],
    pop_count: i16,
}

impl Network {
    pub fn new() -> Self {
        Self {
            input_layer: Layer::new(&INPUT_LAYER_WEIGHT, &INPUT_LAYER_BIAS),
            hidden_layers: [
                Layer::new(&HIDDEN_LAYER_0_WEIGHT, &HIDDEN_LAYER_0_BIAS),
                Layer::new(&HIDDEN_LAYER_1_WEIGHT, &HIDDEN_LAYER_1_BIAS),
                Layer::new(&HIDDEN_LAYER_2_WEIGHT, &HIDDEN_LAYER_2_BIAS),
                Layer::new(&HIDDEN_LAYER_3_WEIGHT, &HIDDEN_LAYER_3_BIAS),
                Layer::new(&HIDDEN_LAYER_4_WEIGHT, &HIDDEN_LAYER_4_BIAS),
                Layer::new(&HIDDEN_LAYER_5_WEIGHT, &HIDDEN_LAYER_5_BIAS),
                Layer::new(&HIDDEN_LAYER_6_WEIGHT, &HIDDEN_LAYER_6_BIAS),
                Layer::new(&HIDDEN_LAYER_7_WEIGHT, &HIDDEN_LAYER_7_BIAS),
            ],
            psqt_layer: Layer::new(&PSQT_LAYER_WEIGHT, &[]),
            accumulator: INPUT_LAYER_BIAS,
            psqt_value: 0,
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
        let feature_idx: usize = piece.index() * SQ::N_SQUARES + sq.index();
        let accumulator_idx = feature_idx * self.input_layer.biases.len();
        let weights = self.input_layer.weights
            [accumulator_idx..accumulator_idx + self.input_layer.biases.len()]
            .iter();

        self.accumulator
            .iter_mut()
            .zip(weights)
            .for_each(|(activation, weight)| update_fn(activation, weight));

        update_fn(&mut self.psqt_value, &self.psqt_layer.weights[feature_idx]);
        update_fn(&mut self.pop_count, &1);
    }

    pub fn eval(&self) -> Value {
        let bucket = (self.pop_count as usize - 1) / 4;

        let hidden_layer = &self.hidden_layers[bucket];
        let output = self
            .accumulator
            .iter()
            .zip(hidden_layer.weights)
            .map(|(&activation, &weight)| Self::clipped_relu(activation) * Value::from(weight))
            .sum::<Value>();

        (Value::from(hidden_layer.biases[0])
            + Value::from(self.psqt_value)
            + output / Self::INPUT_SCALE)
            * Self::NNUE2SCORE
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
