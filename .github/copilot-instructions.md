# Rusty Engine - AI Coding Agent Instructions

## Project Overview
**Rusty Engine** is a chess engine implementation in Rust using bitboard representation for board state and move generation. The project focuses on efficient move generation using magic bitboard lookups and maintains both board state and legal move tracking.

## Architecture

### Core Components

1. **`repr` Module** - Board representation and game logic
   - **`types.rs`**: Defines piece constants (W_PAWN=0...B_KING=11) and `Color` enum
   - **`board.rs`**: `Board` struct with occupation bitboards (`white_occupation`, `black_occupation`) and piece placement
   - **`_move.rs`**: Move encoding as 32-bit integers with bit fields
   - **`bitboard.rs`**: Utility functions for u64 bitboard manipulation (`pop_lsb`, `set_square`, `contains_square`, `bb_to_string` for debugging)
   - **`move_gen.rs`**: `MoveGen` struct with precomputed magic bitboard lookup tables for sliding pieces; methods like `get_all_legal()`, `get_relevant_blockers()`, `get_sliding_for()`
   - **`game.rs`**: `Game` struct tracking board state, move generator, and legal moves; `try_make_move(init_sqr, target_sqr)` updates board and regenerates legal moves

2. **`ui` Module** - GUI layer (using `iced` crate for UI framework)

3. **`utils` Module** - Helper utilities (FEN parsing, etc.)

### Data Flow
1. `Game::init_default()` initializes `Board` (standard starting position) and `MoveGen` (precomputes lookup tables)
2. `move_gen.get_all_legal()` generates legal moves from current board state
3. `Game::try_make_move()` finds matching move in `legal_moves` vec, calls `make_move()` to update board
4. After move, legal moves are regenerated for the new board state

## Key Patterns & Conventions

### Move Encoding
- Moves are **u32 integers**, not structs. Use `_move::create()`, `_move::create_castling()`, `_move::create_promotion()` to encode
- Decode with getter functions: `get_init()`, `get_target()`, `get_moved_piece()`, `is_eating()`, `is_promotion()`, etc.
- Promotion moves require iterating all 4 pieces with `add_all_promotions()` helper

### Bitboard Conventions
- **Relevant blockers**: Only interior squares matter for sliding piece attacks (edges can be ignored). Use `get_relevant_blockers()` to extract only relevant bits
- Magic lookup tables store **with edges** included
- Lookups assume all blockers are enemy pieces; filter with opponent occupation mask when needed
- Use immutable (`bb_to_string()`) and mutable (`pop_lsb()`) variants; immutable variant returns new bb, mutable modifies reference

### Board State
- Board uses parallel bitboards: separate occupation masks for white and black, then per-piece-type masks
- Special constants: `EDGES` for edge squares, `RANKS` for rank detection
- Pieces indexed 0-11: white 0-5, black 6-11 (offset by 6)

## Build & Test

### Commands
- **Build**: `cargo build`
- **Tests**: `cargo test` (or run specific tests like `cargo test naive_slide_gen_works`)
- **Run**: `cargo run` (executes `main.rs`)

### Test Files
- [tests/move_gen_tests.rs](tests/move_gen_tests.rs) - Validates magic bitboard lookups against naive sliding implementations
- [tests/fen_tool_tests.rs](tests/fen_tool_tests.rs) - FEN parsing tests
- [tests/_move_repr_test.rs](tests/_move_repr_test.rs) - Move encoding/decoding tests

### Dependencies
- `iced 0.14.0` - GUI framework
- `rand 0.9.2` - Random number generation
- Rust 2021 edition

## Common Tasks

### Adding a Move Type
1. Define bit field range in `_move.rs` comments at top
2. Add encode function (e.g., `create_*()`) that sets appropriate bit fields
3. Add decode functions (getter + flag checkers like `is_*()`)
4. Update `make_move()` in `game.rs` to handle new move type

### Debugging Board State
- Use `bitboard::bb_to_string()` to visualize any u64 bitboard (prints 8x8 grid)
- Print occupation masks to inspect piece positions
- Main.rs shows examples of iterating moves with `_move::to_string()`

### Performance Considerations
- Hot path: `move_gen.get_all_legal()` and legal move iteration in `try_make_move()`
- Avoid allocating moves; reuse vectors where possible
- Magic bitboards amortize lookup cost: init overhead during `MoveGen::init()`, O(1) per lookup after

## Known Limitations & TODOs
- En passant capture not yet implemented (flagged in README.md)
- Rook/bishop slide lookup tables could be flattened to 1D arrays (optimization)
- Moving king logic currently checks if targets appear in attacked/protected squares (simplification noted in README)
