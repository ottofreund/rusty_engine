- full principal variation search with null windows
- Multithreaded search
- Incremental eval? Should be really easy with additions in make_move & unmake_move
- Search extensions


- Futility pruning, should be easy gains. Also reverse futility pruning?
- Late move reductions
- Killer moves

- More eval heuristics, passed pawn bonus, isolated pawn penalty, controlled squares + other heuristics
- Draw by insufficient material

Others:
- In search if child returns mate score, maybe just return there? Probs not worth it to try to look for shorter mate
- SEE could probably be polished for performance, maybe other approach than maintaining an explicit sorted buffer. Also could try different boundary or dynamic boundary setting
- EP accurate hashing in search too (distinguish legal EP move available from just double push)
- Try out different RNG seeds for Zobrist, can affect a lot
- UCI ponder for GUI