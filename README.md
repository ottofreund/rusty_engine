# Rusty Engine, Chess Engine with a GUI Chess Game on top

## **Rusty Engine** is a W.I.P. chess engine implementation in Rust using bitboard representation for board state and move generation. The project focuses on efficient move generation using magic bitboard lookups.


### Core Components

1. **`repr` Module** - Board representation and game logic
   - **`types.rs`**: Defines piece constants (W_PAWN=0...B_KING=11) and `Color` enum
   - **`board.rs`**: `Board` struct with occupation bitboards (`white_occupation`, `black_occupation`) and piece placement
   - **`_move.rs`**: Move encoding as 32-bit integers with bit fields
   - **`bitboard.rs`**: Utility functions for u64 bitboard manipulation (`pop_lsb`, `set_square`, `contains_square`, `bb_to_string` for debugging)
   - **`move_gen.rs`**: `MoveGen` struct with precomputed magic bitboard lookup tables for sliding pieces; methods like `get_all_legal()`, `get_relevant_blockers()`, `get_sliding_for()`
   - **`game.rs`**: `Game` struct tracking board state, move generator, and legal moves; `try_make_move(init_sqr, target_sqr)` updates board and regenerates legal moves
   - **`magic_bb_loader.rs`**: Finds 'magic numbers' for sliding pieces, and stores them in `MagicBitboard` struct. Also contains precomputed magics for faster startup.

2. **`ui` Module** - GUI layer (using `iced` crate for UI framework)

3. **`utils` Module** - Helper utilities (FEN parsing, etc.)


### Dependencies
- `iced 0.14.0` - GUI framework
- `rand 0.9.2` - Random number generation
- Rust 2021 edition

![alt text](https://github.com/ottofreund/rusty_engine/blob/main/.github/board_demo_img.png "App demo")