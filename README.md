# fluff

Write tiny `gzip` and `brotli` files that expand to one repeated byte — one
byte puffed up into megabytes of nothing. Useful for exercising decompression
limits, streaming buffers, or just seeing how far a compressor will go on the
most boring input possible.

The output is streamed, so generating a multi-gigabyte expansion never uses
more than a few kilobytes of memory.

## Build

```sh
cargo build --release
```

Command-line options are handled by [clap](https://docs.rs/clap).

## Usage

```sh
fluff [OPTIONS]
```

| Option | Description | Default |
| --- | --- | --- |
| `-c, --char <CHAR>` | ASCII byte to repeat | `a` |
| `-s, --size <SIZE>` | decompressed size; suffixes `k`/`m`/`g`/`t` = KiB/MiB/GiB/TiB | `1m` |
| `-o, --out <PREFIX>` | output prefix; writes `<PREFIX>.gz` and/or `<PREFIX>.br` | `repeat` |
| `-f, --format <FORMAT>` | which file(s) to write: `both`, `gz`, or `br` | `both` |
| `-h, --help` | show help | |
| `-V, --version` | show version | |

### Examples

```sh
# Default: 1 MiB of 'a' -> repeat.gz (~1 KB) and repeat.br (far smaller)
fluff

# 1 GiB of '0', written as zero.gz / zero.br
fluff --char 0 --size 1g --out zero

# Only the brotli file (skip the .gz)
fluff --format br --out zero --char 0 --size 1g
```

### Checking the result

```sh
ls -l repeat.gz repeat.br
gunzip -c repeat.gz | wc -c      # -> decompressed byte count
brotli -d -c repeat.br | wc -c   # needs the `brotli` CLI
```

## How it works

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

### Source layout

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

Note that gzip stores the original size in only 32 bits, so for outputs above
4 GiB `gunzip -l` reports the size modulo 2³². The data still decompresses
correctly; `gunzip -c file.gz | wc -c` gives the true size.
