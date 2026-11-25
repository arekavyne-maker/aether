#!/usr/bin/env python3
"""Wrap a square PNG in a modern Windows .ico container (PNG-compressed)."""
import struct
import sys
from pathlib import Path

source = Path(sys.argv[1])
target = Path(sys.argv[2])
png = source.read_bytes()
if png[:8] != b"\x89PNG\r\n\x1a\n":
    raise SystemExit("source is not a PNG")
width, height = struct.unpack(">II", png[16:24])
if width != height:
    raise SystemExit("app icon must be square")
size = min(width, 256)
entry = struct.pack(
    "<BBBBHHII",
    size if size < 256 else 0,
    size if size < 256 else 0,
    0, 0, 1, 32, len(png), 22,
)
target.write_bytes(struct.pack("<HHH", 0, 1, 1) + entry + png)
print(f"wrote {target} ({size}x{size})")
