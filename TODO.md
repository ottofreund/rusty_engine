
- Fifty move counter
- full principal variation search with null windows
- Multithreaded search
- Quiescence search
- Transposition table


- Move selection sort pick in search can be maybe optimized
- Maybe keep prev search PV in search data after moves on the board?

Reminders:
- Maybe clear out pv in search_data objects when starting new search?
- Try out different RNG seeds for Zobrist, can affect a lot
- Instead of multiple position state tracking stacks, maybe put all in same stack under some StateInfo abstraction struct, like in Stockfish

- make search ep hashing more correct with distinguishing positions to those with only ep squares and those with legal ep moves:
     let gained_ep_is_legal = self.board.ep_square.is_some()
    && self.legal_search_moves().iter().any(|m| _move::is_en_passant(*m));
    and check performance diff (maybe not so bad?)