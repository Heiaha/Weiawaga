use crate::evaluation::e_constants::*;
use crate::evaluation::eval::*;
use crate::evaluation::score::Value;
use crate::texel::parameter::Parameter;
use crate::types::board::Board;
use crate::types::piece::*;
use std::fs::File;
use std::io::{BufRead, BufReader};

type GameResult = f64;

pub struct Tuner<'a> {
    parameters: Vec<Parameter<'a>>,
    boards: Vec<Board>,
    results: Vec<GameResult>,
    k: GameResult,
}

impl<'a> Tuner<'a> {
    pub fn new(filename: &str) -> Self {
        let file = File::open(filename).unwrap();
        let reader = BufReader::new(file);

        let mut boards: Vec<Board> = Vec::new();
        let mut results: Vec<GameResult> = Vec::new();
        for (i, line) in reader.lines().enumerate() {
            let line = line.unwrap();

            let (fen, result) = line.rsplit_once(" ").unwrap();
            let numerical_result = match result {
                "[1-0]" | "[1.0]" => 1.0,
                "[1/2-1/2]" | "[0.5]" => 0.5,
                "[0-1]" | "[0.0]" => 0.0,
                _ => {
                    panic!("Line {} with fen {} doesn't contain a result.", i, fen);
                }
            };
            results.push(numerical_result);
            let board = Board::try_from(fen);
            match board {
                Ok(board) => boards.push(board),
                Err(e) => panic!("{}", e),
            }
        }
        println!("Tuning with {} positions.", boards.len());

        let mut parameters: Vec<Parameter> = Vec::new();
        unsafe {
            parameters.push(Parameter::new(&mut TEMPO, "Tempo"));
            parameters.push(Parameter::new(&mut PIECE_TYPE_VALUES, "Piece Type Values"));
            parameters.push(Parameter::new(&mut PAWN_SCORES, "Pawn Scores"));
            parameters.push(Parameter::new(&mut BISHOP_SCORES, "Bishop Scores"));
            parameters.push(Parameter::new(&mut ROOK_SCORES, "Rook Scores"));
            parameters.push(Parameter::new(&mut KING_SCORES, "King Scores"));
            parameters.push(Parameter::new(
                &mut PIECE_TYPE_TABLES[PieceType::Pawn.index()],
                "Pawn PST",
            ));
            parameters.push(Parameter::new(
                &mut PIECE_TYPE_TABLES[PieceType::Knight.index()],
                "Knight PST",
            ));
            parameters.push(Parameter::new(
                &mut PIECE_TYPE_TABLES[PieceType::Bishop.index()],
                "Bishop PST",
            ));
            parameters.push(Parameter::new(
                &mut PIECE_TYPE_TABLES[PieceType::Rook.index()],
                "Rook PST",
            ));
            parameters.push(Parameter::new(
                &mut PIECE_TYPE_TABLES[PieceType::Queen.index()],
                "Queen PST",
            ));
            parameters.push(Parameter::new(
                &mut PIECE_TYPE_TABLES[PieceType::King.index()],
                "King PST",
            ));
        }
        // Find the value of k that minimizes the MSE for the dataset.
        let k = Self::find_best_k(&boards, &results);
        Self {
            parameters,
            boards,
            results,
            k,
        }
    }

    fn find_best_k(boards: &Vec<Board>, results: &Vec<GameResult>) -> GameResult {
        println!("Finding best k.");
        let mut min = -10.0;
        let mut max = 10.0;
        let mut best = Self::mean_squared_error(min, boards, results);
        let mut delta = 1.0;
        for _iteration in 0..10 {
            let mut value = min;
            while value < max {
                let error = Self::mean_squared_error(value, boards, results);
                if error <= best {
                    best = error;
                    min = value;
                }
                value += delta;
            }
            println!("K = {}, E = {}", min, best);
            max = min + delta;
            min = min - delta;
            delta /= 10.0;
        }
        min
    }

    fn mean_squared_error(
        k: GameResult,
        boards: &Vec<Board>,
        results: &Vec<GameResult>,
    ) -> GameResult {
        let mut error: GameResult = 0.0;

        for i in 0..boards.len() {
            let score = tune_eval(&boards[i]) as GameResult;

            let sigmoid = 1.0 / (1.0 + GameResult::powf(10.0, -k * score / 400.0));
            error += GameResult::powf(results[i] - sigmoid, 2.0);
        }
        error /= boards.len() as GameResult;
        error
    }

    pub fn tune(&mut self) {
        println!("Tuning!");
        let mut best_error = Self::mean_squared_error(self.k, &self.boards, &self.results);
        let adjust_value = 1.0 as Value;
        let mut improved = true;
        while improved {
            improved = false;
            for param in &mut self.parameters {
                for phase in 0_usize..=1_usize {
                    for i in 0..param.len() {
                        param.update(i, adjust_value, phase);
                        let mut new_error =
                            Self::mean_squared_error(self.k, &self.boards, &self.results);
                        if new_error < best_error {
                            best_error = new_error;
                            param.mark_best();
                            improved = true;
                            continue;
                        }
                        param.update(i, -adjust_value, phase);
                        new_error = Self::mean_squared_error(self.k, &self.boards, &self.results);
                        if new_error < best_error {
                            best_error = new_error;
                            param.mark_best();
                            improved = true;
                        }
                    }
                }
                println!("Tuned parameter {} with E = {}", param.name, best_error);
            }
            let _clear_file = File::create("results.txt").unwrap();
            for param in &self.parameters {
                param.print_and_save();
            }
        }
    }
}
