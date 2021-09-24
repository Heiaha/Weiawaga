<h1 align="center">Weiawaga</h1>
<p align="center">
  <img src="https://github.com/heiaha/weiawaga/actions/workflows/rust.yml/badge.svg">
</p>

A UCI chess engine written in Rust. If you find this repository, come play me on lichess!

https://lichess.org/@/Weiawaga

## Features

- Board representation
    - [Bitboards](https://en.wikipedia.org/wiki/Bitboard)
- Move generation
    - [Magic bitboard hashing](https://www.chessprogramming.org/Magic_Bitboards)
- Search
    - [Negamax with alpha beta pruning](https://en.wikipedia.org/wiki/Negamax#Negamax_with_alpha_beta_pruning)
    - [Iterative deepening](https://en.wikipedia.org/wiki/Iterative_deepening_depth-first_search)
    - [Quiescence search](https://en.wikipedia.org/wiki/Quiescence_search)
    - [Aspiration windows](https://www.chessprogramming.org/Aspiration_Windows)
    - [Null move pruning](https://www.chessprogramming.org/Null_Move_Pruning)
    - [Check extensions](https://www.chessprogramming.org/Check_Extensions)
- Evaluation
    - [Material imbalance](https://www.chessprogramming.org/Material)
    - [Piece square tables](https://www.chessprogramming.org/Piece-Square_Tables)
    - [Pawn structure](https://www.chessprogramming.org/Pawn_Structure)
    - [King safety](https://www.chessprogramming.org/King_Safety)
    - [Bishop pairs](https://www.chessprogramming.org/Bishop_Pair)
    - [Evaluation tapering](https://www.chessprogramming.org/Tapered_Eval)
    - [Mobility](https://www.chessprogramming.org/Mobility)
    - [Rooks on open file](https://www.chessprogramming.org/Rook_on_Open_File)
- Move ordering
    - [Hash move](https://www.chessprogramming.org/Hash_Move)
    - [MVV/LVA](https://www.chessprogramming.org/MVV-LVA)
    - [Killer heuristic](https://www.chessprogramming.org/Killer_Heuristic)
    - [History heuristic](https://www.chessprogramming.org/History_Heuristic)
- Other
    - [Zobrist hashing](https://www.chessprogramming.org/Zobrist_Hashing) / [Transposition table](https://en.wikipedia.org/wiki/Transposition_table)

Move generation inspired by [surge](https://github.com/nkarve/surge). A previous version of this engine written in Java can be found [here](https://github.com/Heiaha/WeiawagaJ).

**[What's a Weiawaga?](https://www.youtube.com/watch?v=7lRpoYGzx0o)**