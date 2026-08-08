- full principal variation search with null windows
- Multithreaded search
- Incremental eval?


- SEE in quiescence and maybe elsewhere too
- Futility pruning, should be easy gains
- Transposition table
- History heuristic for sorting quiets. Should be easy gains

- Tapered eval
- Draw by insufficient material
- Eval to factor in controlled squares + other heuristics


Others:
- SEE could probably be polished for performance, maybe other approach than maintaining an explicit sorted buffer. Also could try different boundary or dynamic boundary setting
- EP accurate hashing in search too (distinguish legal EP move available from just double push)
- Try out different RNG seeds for Zobrist, can affect a lot
- Previous PV move is still ranked high in ordering even if out of PV line in search, probably not a huge deal but worth to check
- UCI ponder for GUI