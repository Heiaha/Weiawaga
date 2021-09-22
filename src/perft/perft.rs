use crate::types::board::Board;
use crate::types::move_list::MoveList;
use std::time::Instant;

pub fn perft(board: &mut Board, depth: u8) -> usize {
    let mut moves: MoveList = MoveList::new();
    board.generate_legal_moves(&mut moves);

    if depth == 1 {
        return moves.len();
    }

    let mut nodes: usize = 0;

    for m in moves {
        board.push(m);
        nodes += perft(board, depth - 1);
        board.pop();
    }
    nodes
}

pub fn print_perft(board: &mut Board, depth: u8) -> usize {
    let mut moves: MoveList = MoveList::new();
    board.generate_legal_moves(&mut moves);

    let mut nodes: usize = 0;
    let now = Instant::now();
    for m in moves {
        print!("{} ", m.to_string());
        board.push(m);
        let move_nodes = perft(board, depth - 1);
        board.pop();
        nodes += move_nodes;
        println!("{}", move_nodes);
    }
    let elapsed = now.elapsed().as_millis() as f64 / 1000.0 as f64;
    if elapsed > 0_f64 {
        println!("Elapsed: {}", elapsed);
        println!("NPS: {:.1}", nodes as f64 / elapsed);
    }
    println!("Nodes: {}", nodes);
    nodes
}
