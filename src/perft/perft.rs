use crate::types::board::*;
use crate::types::move_list::*;
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
        print!("{}: ", m.to_string());
        board.push(m);
        let move_nodes = perft(board, depth - 1);
        board.pop();
        nodes += move_nodes;
        println!("{}", move_nodes);
    }
    let elapsed = now.elapsed().as_millis() as f64 / 1000.0_f64;
    println!();
    println!("{}", board);
    println!("FEN: {}", board.fen());
    println!("Hash: {}", board.hash());
    println!("Nodes: {}", nodes);
    if elapsed > 0_f64 {
        println!("NPS: {:.0}", nodes as f64 / elapsed);
        println!("Elapsed: {} seconds", elapsed);
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::attacks::init_attacks;
    use crate::types::bitboard::init_bb;
    use crate::types::magics::init_magics;

    #[test]
    fn test_perft() {
        init_magics();
        init_attacks();
        init_bb();

        assert_eq!(perft(&mut Board::new(), 5), 4865609);
        assert_eq!(perft(&mut Board::from("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - "), 4), 4085603);
        assert_eq!(perft(&mut Board::from("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - "), 5), 674624);
        assert_eq!(perft(&mut Board::from("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1"), 4), 422333);
        assert_eq!(perft(&mut Board::from("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8"), 4), 2103487);
        assert_eq!(perft(&mut Board::from("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10"), 5), 164075551);
    }
}
