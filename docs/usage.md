# Using fluff

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

## Examples

```sh
# Default: 1 MiB of 'a' -> repeat.gz (~1 KB) and repeat.br (13 bytes)
fluff

# 1 GiB of '0', written as zero.gz / zero.br
fluff --char 0 --size 1g --out zero

# Only the brotli file (skip the .gz)
fluff --format br --out zero --char 0 --size 1g
```

The make tasks run the same command and put the files in `out/`:
`make files C=0 SIZE=1g NAME=zero FORMAT=br`.

## Checking the result

```sh
ls -l repeat.gz repeat.br
gunzip -c repeat.gz | wc -c      # -> decompressed byte count
brotli -d -c repeat.br | wc -c   # needs the `brotli` CLI
```

`make verify` does the same for the files in `out/`.

gzip stores the original size in only 32 bits, so for outputs above 4 GiB
`gunzip -l` reports the size modulo 2³². The data still decompresses
correctly; `gunzip -c file.gz | wc -c` gives the true size.

---

[Back to the README](../README.md)
