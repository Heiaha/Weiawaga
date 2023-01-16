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
    - [Fancy magic bitboard hashing](https://www.chessprogramming.org/Magic_Bitboards#Fancy)
- Search
    - [Principal variation search](https://www.chessprogramming.org/Principal_Variation_Search)
    - [Lazy SMP](https://www.chessprogramming.org/Lazy_SMP)
    - [Iterative deepening](https://en.wikipedia.org/wiki/Iterative_deepening_depth-first_search)
    - [Quiescence search](https://en.wikipedia.org/wiki/Quiescence_search)
    - [Aspiration windows](https://www.chessprogramming.org/Aspiration_Windows)
    - [Reverse futility pruning](https://www.chessprogramming.org/Reverse_Futility_Pruning)
    - [Null move pruning](https://www.chessprogramming.org/Null_Move_Pruning)
    - [Check extensions](https://www.chessprogramming.org/Check_Extensions)
- [NNUE](https://www.chessprogramming.org/NNUE) evaluation
    - Incremental updating 
    - Buckets
- Move ordering
    - [Hash move](https://www.chessprogramming.org/Hash_Move)
    - [Static exchange evaluation](https://www.chessprogramming.org/Static_Exchange_Evaluation)
    - [Killer heuristic](https://www.chessprogramming.org/Killer_Heuristic)
    - [History heuristic](https://www.chessprogramming.org/History_Heuristic)
- Other
    - [Zobrist hashing](https://www.chessprogramming.org/Zobrist_Hashing) / [Transposition table](https://en.wikipedia.org/wiki/Transposition_table)

Move generation inspired by [surge](https://github.com/nkarve/surge). A previous version of this engine written in Java can be found [here](https://github.com/Heiaha/WeiawagaJ).
The NNUE training code can be found [here](https://github.com/Heiaha/Mimir).

**[What's a Weiawaga?](https://www.youtube.com/watch?v=7lRpoYGzx0o)**