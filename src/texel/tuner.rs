use std::fs::File;
use std::io::{BufRead, BufReader};
use std::mem;
use crate::texel::parameter::Parameter;
use crate::types::board::Board;
use crate::evaluation::e_constants::*;
use crate::types::piece::*;
use crate::evaluation::eval::*;
use crate::evaluation::score::Value;
use crate::types::move_list::MoveList;


type GameResult = f64;

pub struct Tuner<'a> {
    parameters: Vec<Parameter<'a>>,
    boards: Vec<Board>,
    results: Vec<GameResult>,
    k: GameResult,
}

impl<'a> Tuner<'a> {
    pub fn new(filename: String) -> Self {
        let file = File::open(filename).unwrap();
        let reader = BufReader::new(file);

        let mut boards: Vec<Board> = Vec::new();
        let mut results: Vec<GameResult> = Vec::new();
        for (i, line) in reader.lines().enumerate() {
            let mut line = line.unwrap();

            let result = if line.contains("[1-0]") {
                line = line.replace("[1-0]", "");
                1.0 as GameResult
            } else if line.contains("[1/2-1/2]") {
                line = line.replace("[1/2-1/2]", "");
                0.5 as GameResult
            } else if line.contains("[0-1]") {
                line = line.replace("[0-1]", "");
                0.0 as GameResult
            } else {
                panic!("Line doesn't contain a result!");
            };
            results.push(result);
            let board = Board::from(&*line);
            boards.push(board);
        }

        let mut parameters: Vec<Parameter> = Vec::new();
        unsafe {
            parameters.push(Parameter::new(&mut TEMPO, "Tempo"));
            parameters.push(Parameter::new(&mut PIECE_TYPE_VALUES, "Piece Type Values"));
            parameters.push(Parameter::new(&mut PAWN_SCORES, "Pawn Scores"));
            parameters.push(Parameter::new(&mut BISHOP_SCORES, "Bishop Scores"));
            parameters.push(Parameter::new(&mut ROOK_SCORES, "Rook Scores"));
            parameters.push(Parameter::new(&mut KING_SCORES, "King Scores"));
            parameters.push(Parameter::new(&mut PIECE_TYPE_TABLES[PieceType::Pawn.index()], "Pawn PST"));
            parameters.push(Parameter::new(&mut PIECE_TYPE_TABLES[PieceType::Knight.index()], "Knight PST"));
            parameters.push(Parameter::new(&mut PIECE_TYPE_TABLES[PieceType::Bishop.index()], "Bishop PST"));
            parameters.push(Parameter::new(&mut PIECE_TYPE_TABLES[PieceType::Rook.index()], "Rook PST"));
            parameters.push(Parameter::new(&mut PIECE_TYPE_TABLES[PieceType::Queen.index()], "Queen PST"));
            parameters.push(Parameter::new(&mut PIECE_TYPE_TABLES[PieceType::King.index()], "King PST"));
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
        let mut k: GameResult = 1.0;
        let mut min: GameResult = -10.0;
        let mut max: GameResult = 10.0;
        let mut delta: GameResult = 1.0;
        let mut best_k: GameResult = k;
        let mut best_error: GameResult = 100.0;
        for iteration in 0..10 {
            while min < max {
                k = min;
                let new_error = Self::mean_squared_error(k, &boards, &results);
                println!("k = {} with MSE = {}, min = {}, max = {}", best_k, best_error, min, max);
                if new_error < best_error {
                    best_error = new_error;
                    best_k = k;
                }
                min += delta;
            }
            min = best_k - delta;
            max = best_k + delta;
            delta /= 10.0;
        }
        best_k
    }

    fn mean_squared_error(K: GameResult, boards: &Vec<Board>, results: &Vec<GameResult>) -> GameResult {
        let mut error: GameResult = 0.0;

        for i in 0..boards.len() {
            let score = tune_eval(&boards[i]) as GameResult;

            let sigmoid = 1.0 / (1.0 + GameResult::powf(10.0, -K*score/400.0));
            error += GameResult::powf(results[i] - sigmoid, 2.0);
        }
        error /= boards.len() as GameResult;
        error
    }

    pub fn tune(&mut self) {
        println!("Tuning!");
        let mut best_error = Self::mean_squared_error(self.k, &self.boards, &self.results);
        let mut adjust_value = 1.0 as Value;
        let mut improved = true;
        while improved {
            let clear_file = File::create("results.txt").unwrap();
            improved = false;
            for param in &mut self.parameters {
                for phase in 0_usize..=1_usize {
                    for i in 0..param.len() {

                        param.update(i, adjust_value, phase);
                        let mut new_error = Self::mean_squared_error(self.k, &self.boards, &self.results);
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
                println!("Tuned parameter {} with MSE = {}", param.name, best_error);
                param.print_and_save();
            }
        }
    }
}
