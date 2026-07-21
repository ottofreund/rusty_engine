- Futility pruning, should be easy gains
- Quiescence search with SEE and Delta pruning
- full principal variation search with null windows
- Multithreaded search
- Transposition table

- Draw by insufficient material
- UCI ponder
- Triangular PV table to avoid allocation at every node

Reminders:
- Try out different RNG seeds for Zobrist, can affect a lot
- Previous PV move is still ranked high in ordering even if out of PV line in search, probably not a huge deal but worth to check
- In quiescence if in check, probably generate more than just captures so that doesn't think it's mate when it's not

- 