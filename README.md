# Rusty Engine

Rusty Engine is a work-in-progress chess engine written in Rust. It combines a
bitboard-based chess core, perft-tested legal move generation, iterative-deepening
principal-variation search, and a UCI command-line interface.


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
  search (PVS), aspiration windows, and mate-distance pruning
- Cooperative cancellation and UCI diagnostics for depth, selective depth,
  score, elapsed time, node count, NPS, and the principal variation

### Selectivity

- Capture-and-promotion quiescence search with stand-pat pruning and full move
  generation in check, with stalemate detection and guarded capture pruning
- Late-move reductions adjusted for move history, checks, and principal variation,
  with a full-depth re-search when a reduced move improves alpha
- Adaptive null-move pruning, razoring, reverse futility, and late quiet move pruning
- Check extensions for principal variations and forced evasions, plus singular
  extensions for verified transposition-table moves

### Search enhancements and move ordering

- Principal-variation and transposition-table move priority, threshold-based static
  exchange evaluation, killer moves, and capture, quiet, and continuation history
- A configurable 16 MB-by-default, cache-line-aligned, clustered transposition
  table with depth-, bound-, and generation-aware replacement and search cutoffs

### Evaluation

- Game-phase-aware material, piece-square, pawn structure, mobility, king safety,
  outpost, rook activity, and threat evaluation
- Endgame scaling, guidance for mating a bare king, and a built-in king-and-pawn
  versus king bitbase
- Pawn-structure correction history that adjusts static evaluation using search results

### Interfaces

- A UCI front end with adaptive clock management, fixed-time and fixed-depth
  search, infinite search, pondering, configurable Hash and Move Overhead, and
  board display with `d`
- An `iced` board for player-versus-engine games and FEN loading, with its image
  and evaluation assets embedded in the binary

## Quick Start

Requires a current stable Rust toolchain with Cargo.

```sh
git clone https://github.com/ottofreund/rusty_engine.git
cd rusty_engine
cargo run --release
```

To run the deterministic OpenBench workload and exit, use:

```sh
cargo run --release -- bench
```

It searches five fixed positions at depth 6 with one thread and emits a final
`<nodes> nodes <nps> nps` line suitable for OpenBench.

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

Supported search modes include `go depth <plies>`, `go movetime <milliseconds>`,
`go infinite`, and clock-based `go` commands using `wtime`, `btime`, optional
`winc` and `binc`, and optional `movestogo`. `go ponder` starts a pondering search,
and `ponderhit` starts its clock. Positions
may use `startpos` or a FEN followed by optional UCI moves. `ucinewgame` clears
search state between games, while `stop` cooperatively cancels an active search.
The non-standard `d` command prints the current board. The UCI `Hash` spin option
configures the transposition table in MB, with a default of 16 and an accepted
range of 1–32768. `Move Overhead` reserves time for communication and scheduling,
with a default of 15 ms and a range of 0–5000 ms.

The desktop GUI is available at the with-gui branch. The GUI supports player-side selection, player-versus-engine play,
promotion selection, FEN loading, legal-move highlighting, and game-over dialogs.

## Architecture

- **`repr`** is the chess core: bitboards, compact moves, `Board` state,
  reversible `Position` state, magic sliding attacks, and legal move generation.
- **`search`** owns the PVS/LMR searcher, per-search state and configuration,
  evaluation, static exchange evaluation, move ordering, and the transposition
  table.
- **`game`** provides `Game` for tracked player-versus-engine games and `CpuGame`
  for importing or incrementally synchronizing UCI positions.
- **`uci`** parses commands, manages UCI options, and runs cancellable searches
  on a worker thread;
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
cargo test --release --test search_benchmark -- --ignored --test-threads=1 --show-output
cargo test --release --test move_gen_benchmark -- --ignored --test-threads=1 --show-output
```

Run benchmarks in release mode and one at a time to avoid CPU contention between
tests. Search diagnostics are disabled during measurements; benchmark summaries
are printed afterward.

## Remaining Work

Planned work includes incremental evaluation, further search and evaluation
tuning, large-page allocation for the transposition table, and multithreaded
search.
