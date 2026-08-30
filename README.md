# Rusty Engine

Rusty Engine is a work-in-progress chess engine written in Rust. It combines a
bitboard-based chess core, perft-tested legal move generation, iterative-deepening
principal-variation search, a UCI command-line interface, and an
[`iced`](https://iced.rs/) desktop GUI.

![Rusty Engine board](.github/board_demo_img.png)

## Features

### Move generation and position state

- Bitboard board representation and magic-bitboard sliding attacks
- Legal move generation for checks, pins, castling, en passant, and promotions
- Compact `u32` moves, reversible make/unmake state, and FEN loading with
  king-count, castling-right, and en-passant consistency checks
- Incremental Zobrist hashing with threefold repetition and fifty-move-rule
  handling in both games and search

### Search

- Fixed-depth or timed iterative-deepening negamax with principal variation
  search (PVS) and alpha-beta pruning
- Cooperative cancellation and UCI diagnostics for depth, selective depth,
  score, node and cutoff counts, and the principal variation

### Selectivity

- Capture-and-promotion quiescence search with stand-pat pruning and full move
  generation when the side to move is in check
- Logarithmic late-move reductions (LMR), with a full-depth re-search when a
  reduced move improves alpha

### Search enhancements and move ordering

- Previous-principal-variation and transposition-table move priority, static
  exchange evaluation for captures, and an aged history heuristic for quiet moves
- A configurable 16 MB-by-default, cache-line-aligned, clustered transposition
  table with depth-, bound-, and generation-aware replacement and search cutoffs

### Evaluation

- Material and piece-square evaluation, with game-phase interpolation between
  opening and endgame pawn and king tables

### Interfaces

- A UCI front end for position import, `ucinewgame`, clock-based, fixed-time, or
  fixed-depth search, Hash configuration, `stop`, board display with `d`, and
  `bestmove` output with a principal-variation-derived `ponder` move
- An `iced` board for player-versus-engine games and FEN loading, with its image
  and evaluation assets embedded in the binary

## Quick Start

Requires a current stable Rust toolchain with Cargo.

```sh
git clone https://github.com/ottofreund/rusty_engine.git
cd rusty_engine
cargo run --release
```

The default executable starts the stdin/stdout UCI interface. For example, after
launching it, enter:

```text
uci
setoption name Hash value 64
isready
position startpos moves e2e4 e7e5
go depth 6
quit
```

Supported search modes are `go depth <plies>`, `go movetime <milliseconds>`, or
clock-based `go` commands using `wtime`, `btime`, `winc`, and `binc`. Positions
may use `startpos` or a FEN followed by optional UCI moves. `ucinewgame` clears
search state between games, while `stop` cooperatively cancels an active search.
The non-standard `d` command prints the current board. The UCI `Hash` spin option
configures the transposition table in MB, with a default of 16 and an accepted
range of 1–32768.

The desktop GUI is still available, but there is not yet a runtime front-end
selector. Set `uci_mode` to `false` in `src/main.rs` and run the command above to
launch it. The GUI supports player-side selection, player-versus-engine play,
promotion selection, FEN loading, legal-move highlighting, and game-over
dialogs.

## Architecture

- **`repr`** is the chess core: bitboards, compact moves, `Board` state,
  reversible `Position` state, magic sliding attacks, and legal move generation.
- **`search`** owns the PVS/LMR searcher, per-search state and configuration,
  evaluation, static exchange evaluation, move ordering, and the transposition
  table.
- **`game`** provides `Game` for tracked player-versus-engine games and `CpuGame`
  for importing or incrementally synchronizing UCI positions.
- **`uci`** parses commands, manages UCI options, and runs cancellable searches
  on a worker thread; **`ui`** contains the `iced` desktop front end.
- **`utils`** provides FEN conversion and Zobrist hashing; `main.rs` currently
  selects between the UCI and GUI front ends with a source-level flag.

## Testing

```sh
cargo test -- --test-threads=1
```

The suite covers move encoding, FEN handling, UCI parsing, move generation and
perft, evaluation, static exchange evaluation, search and cancellation,
transposition-table behavior, incremental Zobrist hashing, threefold repetition,
and special-move make/unmake round trips. Perft and timing tests can take
noticeably longer than the other tests.

Ignored fixed-depth, consecutive-search, and timed-search benchmarks can be run
with:

```sh
cargo test --release --test search_benchmark -- --ignored --show-output
```

## Remaining Work

Planned work includes multithreaded search; search extensions; futility and
reverse-futility pruning; incremental evaluation and richer positional terms;
large-page allocation for the transposition table; draw detection for
insufficient material; and full UCI pondering with `go ponder` and `ponderhit`.
