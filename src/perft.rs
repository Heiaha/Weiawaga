use std::time::Instant;

use super::board::*;
use super::move_list::*;
use super::types::*;

pub fn perft(board: &mut Board, depth: Depth) -> u128 {
    let moves: MoveList = MoveList::from(board);

    if depth == 1 {
        return moves.len() as u128;
    }

    let mut nodes = 0;

    for m in moves {
        board.push(m);
        nodes += perft(board, depth - 1);
        board.pop();
    }
    nodes
}

pub fn print_perft(board: &mut Board, depth: Depth) -> u128 {
    let now = Instant::now();

    let moves: MoveList = MoveList::from(board);
    let mut nodes = 0;
    for m in moves {
        print!("{}: ", m);
        board.push(m);
        let move_nodes = perft(board, depth - 1);
        board.pop();
        nodes += move_nodes;
        println!("{}", move_nodes);
    }
    let elapsed = now.elapsed().as_millis() / 1000;
    println!();
    println!("{:?}", board);
    println!("FEN: {}", board);
    println!("Hash: {}", board.hash());
    println!("Nodes: {}", nodes);
    if elapsed > 0 {
        println!("NPS: {:.0}", nodes / elapsed);
        println!("Elapsed: {:.1} seconds", elapsed);
    }
    nodes
}

#[cfg(test)]
mod tests {
    use crate::bitboard::init_bb;
    use crate::magics::init_magics;
    use crate::perft::perft;

    use super::*;

    #[test]
    fn test_perft() {
        init_magics();
        init_bb();

        assert_eq!(perft(&mut Board::new(), 5), 4865609);
        assert_eq!(
            perft(
                &mut Board::try_from(
                    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - "
                )
                .unwrap(),
                4
            ),
            4085603
        );
        assert_eq!(
            perft(
                &mut Board::try_from("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - ").unwrap(),
                5
            ),
            674624
        );
        assert_eq!(
            perft(
                &mut Board::try_from(
                    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1"
                )
                .unwrap(),
                4
            ),
            422333
        );
        assert_eq!(
            perft(
                &mut Board::try_from("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8")
                    .unwrap(),
                4
            ),
            2103487
        );
        assert_eq!(
            perft(
                &mut Board::try_from(
                    "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10"
                )
                .unwrap(),
                5
            ),
            164075551
        );
    }
}
