# Strict Total Commander UTF-8 Encoding Only

Dion exclusively reads and writes `descript.ion` files adhering to the Total Commander UTF-8 standard (prefixed with `0xEFBBBF0D0A`). Any pre-existing `descript.ion` with ANSI or UTF-16 encoding will trigger an explicit error rather than attempting heuristic auto-detection or in-place transcoding, avoiding accidental data corruption or encoding mismatch across legacy tools.
