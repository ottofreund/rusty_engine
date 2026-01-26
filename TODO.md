Move generation todo:
 - en passant

Easy improvements:
 - flatten rook_slide_bbs and bishop_slide_bbs to 1D


Reminders:
 - en passant edge case to self discovered check
 - Attacked squares and protected squares actually don't have to be distinguished. Attacked squares, that are generated from pseudolegals, can simply be allowed to contain "eat own piece / protect" moves and thus when moving king we just have to check that these pseudolegal targets <==> attacked/protected squares don't contain the king target.
 - Lookups to sliding piece arrays are done with **relevant** blocker bitboards, meaning that last square before edge is dismissed. This reduces the combination space greatly with minimal overhead of converting to relevant bb. The values of these lookup tables are naturally stored **with** edges.
 - Sliding piece lookups are computed with assumption that all blockers are enemy pieces. This assumption is easy to relieve using enemy piece occupation bb mask.


Checklist:
 - Is having eaten piece type necessary in move integer? If no need, can make hot code paths more efficient with changing taken piece type -> piece taken flag

