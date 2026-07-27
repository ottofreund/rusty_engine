- full principal variation search with null windows
- Multithreaded search
- Incremental eval?


- SEE in quiescence and maybe elsewhere too
- Futility pruning, should be easy gains
- Transposition table

- Tapered eval
- Draw by insufficient material
- Eval to factor in controlled squares + other heuristics


Others:
- Try out different RNG seeds for Zobrist, can affect a lot
- Previous PV move is still ranked high in ordering even if out of PV line in search, probably not a huge deal but worth to check
- UCI ponder for GUI