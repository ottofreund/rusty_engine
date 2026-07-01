
- Search is slower due to repetition map because it fills up with ton of entries with value 0 that are never removed
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