# Python archive reference fixtures

Produced with the unmodified `engines/k1tty_pack.py` from upstream commit
`b5894dae5feaf13947c2d49a34f91b088cde60ec`.

`python_fast.k1tty`, `python_balanced.k1tty`, and `python_ultra.k1tty` use the
original preset settings and Zstandard. `python_lzma2.k1tty` was produced with
the original fallback path (`_HAS_ZSTD = False`). `expected.json` contains the
normalized metadata and entries returned by the original Python decoder.

The fixture includes Cyrillic, Ukrainian, emoji, repeated bodies spanning several
64 KiB chunks, millisecond timestamps, missing timestamps, an empty body, tool
metadata, an error code, and nested metadata. Rust tests compare every decoded
entry and metadata value against that independent reference.
