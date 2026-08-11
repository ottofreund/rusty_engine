- full principal variation search with null windows
- Multithreaded search
- Incremental eval? Should be really easy with additions in make_move & unmake_move
- Use partial search results after interrupting search, no point in discarding since guaranteed better than previous depth complete search
- Search extensions


- Futility pruning, should be easy gains
- Transposition table
- History heuristic for sorting quiets. Should be easy gains
- Late move reductions

- Tapered eval
- More eval heuristics, passed pawn bonus, isolated pawn penalty
- Draw by insufficient material
- Eval to factor in controlled squares + other heuristics


Others:
- SEE could probably be polished for performance, maybe other approach than maintaining an explicit sorted buffer. Also could try different boundary or dynamic boundary setting
- EP accurate hashing in search too (distinguish legal EP move available from just double push)
- Try out different RNG seeds for Zobrist, can affect a lot
- Previous PV move is still ranked high in ordering even if out of PV line in search, probably not a huge deal but worth to check
- UCI ponder for GUI