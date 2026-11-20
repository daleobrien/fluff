# How it works

Because the input is a single byte repeated, there is nothing to search for —
the compressed form is written directly:

- **gzip** is hand-written (no compression library). The first byte is emitted
  as a literal, followed by a run of length-258, distance-1 back-references. A
  custom dynamic-Huffman block gives those length and distance symbols one-bit
  codes, which reaches DEFLATE's theoretical ceiling. The CRC-32 of the (huge)
  output is computed by CRC *combination* in O(log n), so even a terabyte is
  instant to checksum.
- **brotli** is hand-written too (no compression library). Each meta-block
  emits the byte as a single literal followed by one copy of up to ~16 MiB at
  distance 1. All three prefix codes (literal, insert-and-copy, and distance)
  are *simple* one-symbol codes, so their symbols cost zero bits — a whole
  meta-block is a handful of header bits plus the copy length's extra bits.
  Distance 1 uses a direct distance code, so each meta-block is self-contained;
  outputs past brotli's 16 MiB meta-block limit are split into several
  identical meta-blocks and stream in flat memory.

## Source layout

```
src/
  main.rs            entry point and the run/report glue
  cli.rs             clap argument definitions and value parsers
  report.rs          size/ratio formatting
  brotli/
    mod.rs           assembles the brotli stream
    bitwriter.rs     LSB-first bit writer
    tables.rs        insert/copy length codes + command mapping
  gzip/
    mod.rs           assembles the gzip stream
    bitwriter.rs     LSB-first bit/Huffman-code writer
    huffman.rs       Huffman code construction + code-length RLE
    crc.rs           CRC-32 via CRC combination
```

## A note on ratios

`gzip` (DEFLATE) has a hard ceiling near **1032:1**, because each back-reference
copies at most 258 bytes. So ~1 MiB of a single byte is about the smallest input
that still produces a roughly 1 KB `.gz` — go much smaller and the gzip file
shrinks below a kilobyte; go larger and it grows linearly (~1 KB per MiB).

`brotli` can copy up to ~16 MiB in a single command (against gzip's 258 bytes),
so one meta-block turns 16 MiB of output into about 13 bytes — a ratio over a
million to one — and it holds that as the output grows.

## The cover

[`cover.png`](cover.png) shows the real 13-byte `repeat.br` from `make files`
in a sea of the letter `a`. `cover.py` draws it: `python3 docs/cover.py
docs/cover.svg`, then `rsvg-convert -w 1280 -h 640 docs/cover.svg -o
docs/cover.png`.

---

[Back to the README](../README.md)
