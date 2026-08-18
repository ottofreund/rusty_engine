- full principal variation search with null windows
- Multithreaded search
- Incremental eval? Should be really easy with additions in make_move & unmake_move
- Search extensions


- Futility pruning, should be easy gains. Also reverse futility pruning?
- Transposition table
- Late move reductions
- Killer moves

- Tapered eval
- More eval heuristics, passed pawn bonus, isolated pawn penalty
- Draw by insufficient material
- Eval to factor in controlled squares + other heuristics


Others:
- Don't use TT if close to hitting 50 move rule, also three-fold
- To mitigate TT collision and maintain correctness, check is_pseudolegal(tt_move) && pseudolegal_is_legal(tt_move)
- SEE could probably be polished for performance, maybe other approach than maintaining an explicit sorted buffer. Also could try different boundary or dynamic boundary setting
- EP accurate hashing in search too (distinguish legal EP move available from just double push)
- Try out different RNG seeds for Zobrist, can affect a lot
- UCI ponder for GUI