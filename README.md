<h1 align="center">Weiawaga</h1>
<p align="center">
  <img src="https://github.com/heiaha/weiawaga/actions/workflows/rust.yml/badge.svg">
  <img src="https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2FHeiaha%2FWeiawaga%2Fmaster%2FCargo.toml&query=package.rust-version&label=rust&logo=rust&color=orange">
  <img src="https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Flichess.org%2Fapi%2Fuser%2FWeiawaga&query=%24.perfs.blitz.rating&label=lichess%20blitz&logo=lichess&color=success">
  <img src="https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Flichess.org%2Fapi%2Fuser%2FWeiawaga&query=%24.count.all&label=games%20played&logo=lichess&color=blue">
</p>

A UCI chess engine written in Rust. If you find this repository, come play me on lichess!

https://lichess.org/@/Weiawaga

## Overview

Weiawaga is built on bitboards with fancy magic move generation and searches
with a principal variation alpha-beta framework. It includes iterative deepening,
aspiration windows, a lockless transposition table, and Lazy SMP parallelism,
with the usual modern selectivity on top. Evaluation is a hand-rolled
efficiently updatable neural network (NNUE) with SIMD inference, trained with
[Mimir](https://github.com/Heiaha/Mimir).

## Building

Requires stable Rust (1.97+). The network weights are embedded in the binary
at compile time, so a build is fully self-contained:

```
cargo build --release
```

## Usage

Weiawaga speaks standard UCI: point any UCI-compatible interface (cutechess,
Arena, Banksia, En Croissant, ...) at the binary, or drive it directly from a
terminal. Supported options:

| Option          | Description                                        |
| --------------- | -------------------------------------------------- |
| `Hash`          | Transposition table size in MB                     |
| `Threads`       | Number of search threads                           |
| `MultiPV`       | Number of principal variations to report           |
| `Ponder`        | Think on the opponent's time                       |
| `Move Overhead` | Per-move time buffer (ms) for connection latency   |
| `UCI_ShowWDL`   | Report win/draw/loss probabilities with the score  |
| `Clear Hash`    | Clear the transposition table                      |

## Acknowledgements

Move generation inspired by [surge](https://github.com/nkarve/surge). A
previous version of this engine written in Java can be found
[here](https://github.com/Heiaha/WeiawagaJ). The NNUE training code can be
found [here](https://github.com/Heiaha/Mimir).

**[What's a Weiawaga?](https://www.youtube.com/watch?v=7lRpoYGzx0o)**
