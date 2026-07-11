# Rusty Engine

Rusty Engine is a work-in-progress chess engine written in Rust. It combines a
bitboard-based chess core, perft-tested legal move generation, iterative-deepening
alpha-beta search, an [`iced`](https://iced.rs/) desktop GUI, and an experimental
UCI interface.

![Rusty Engine board](.github/board_demo_img.png)

## Features

- Bitboard board representation and magic-bitboard sliding attacks
- Legal move generation for checks, pins, castling, en passant, and promotions
- Compact `u32` moves, reversible make/unmake state, and FEN loading with
  king-count, castling-right, and en-passant consistency checks
- Incremental Zobrist hashing with threefold repetition and fifty-move-rule
  handling in both games and search
- Opening/endgame piece-square evaluation and fixed-depth or timed
  iterative-deepening negamax search with alpha-beta pruning
- Move ordering and consecutive-search reuse based on the principal variation,
  plus cooperative cancellation for timed searches
- An `iced` board for manual play and FEN loading, with its image assets embedded
  in the binary
- A partial UCI listener for position import, clock-based or fixed-time search,
  `stop`, and `bestmove`

## Quick Start

Requires a current stable Rust toolchain with Cargo.

```sh
git clone https://github.com/ottofreund/rusty_engine.git
cd rusty_engine
cargo run --release
```

The default executable opens the desktop GUI. It supports manual play, FEN
loading, and manually triggered searches; automatic engine moves in GUI games
are not implemented yet.

The codebase also contains a partial stdin/stdout UCI listener, although it is
not wired to the default executable. It handles `uci`, `isready`, `position`
with a starting position or FEN and optional moves, timed `go`, `stop`, and
`quit`. Options, new-game reset behavior, and pondering remain unfinished.

## Architecture

- **`repr`** contains `Board`, `Position`, compact moves, and the legal move
  generator.
- **`search`** contains the evaluator, search configuration, repetition history,
  iterative deepening, move ordering, and cancellation logic.
- **`game`** provides `Game` for on-board state and `CpuGame` for importing and
  synchronizing UCI positions.
- **`utils`**, **`ui`**, and **`uci`** provide FEN/Zobrist utilities and the two
  current front ends.

## Testing

```sh
cargo test -- --test-threads=1
```

The suite covers move encoding, FEN handling, move generation and perft,
evaluation, incremental Zobrist hashing, threefold repetition, and special-move
make/unmake round trips. Perft tests visit millions of positions and can take
noticeably longer than the other tests.

Ignored fixed-depth, consecutive-search, and timed-search benchmarks can be run
with:

```sh
cargo test --release --test search_benchmark -- --ignored --show-output
```

## Remaining Work

Planned work includes automatic engine play in the GUI, fuller UCI support,
principal-variation search with null windows, quiescence search, a transposition
table, and multithreaded search.
