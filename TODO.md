- SEE in quiescence and maybe elsewhere too
- Tapered eval
- Futility pruning, should be easy gains
- full principal variation search with null windows
- Multithreaded search
- Transposition table

- Draw by insufficient material
- Triangular PV table to avoid allocation at every node
- Eval to factor in controlled squares + other heuristics
- Incremental eval?
- UCI ponder

Reminders:
- Turn delta pruning off in late endgame
- Try out different RNG seeds for Zobrist, can affect a lot
- Previous PV move is still ranked high in ordering even if out of PV line in search, probably not a huge deal but worth to check