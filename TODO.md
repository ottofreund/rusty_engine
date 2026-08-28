- Multithreaded search
- Incremental eval? Should be really easy with additions in make_move & unmake_move
- Search extensions
- Large pages in TT


- Futility pruning, should be easy gains. Also reverse futility pruning?
- Late move reductions

- More eval heuristics, passed pawn bonus, isolated pawn penalty, controlled squares + other heuristics
- Draw by insufficient material

Others:
- Maybe lower half-move-clock condition for TT cutoffs from 96 to something even lower cuz still repetition three-fold blindness
- SEE could probably be polished for performance, maybe other approach than maintaining an explicit sorted buffer. Also could try different boundary or dynamic boundary setting
- Try out different RNG seeds for Zobrist, can affect a lot
- UCI ponder for GUI