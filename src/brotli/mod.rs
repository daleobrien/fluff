//! A tiny brotli writer specialised for one repeated byte.
//!
//! Like the gzip side, the input is a single byte repeated `total` times, so
//! there is nothing to "search". The output is written directly: each
//! meta-block emits the byte once as a literal and then a single copy of up to
//! ~16 MiB at distance 1. All three prefix codes (literal, insert-and-copy,
//! distance) are *simple* one-symbol codes, so their symbols cost zero bits —
//! the entire expansion of one meta-block is a handful of header bits plus the
//! copy-length's extra bits.
//!
//! brotli caps a meta-block at 2^24 bytes, so an output larger than 16 MiB is
//! split into several identical meta-blocks. Distance 1 is encoded with a
//! direct distance code (code 16, one extra bit 0) rather than the distance
//! cache, so every meta-block is self-contained and the stream never needs to
//! track state. Everything here is `std`-only; this module needs no crate.
//!
//! Reference: RFC 7932 (Brotli Compressed Data Format).

mod bitwriter;
mod tables;

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};

use bitwriter::BitWriter;
use tables::{combine, copy_code, insert_code, COPY, INSERT};

/// Window size exponent: window = (1 << WBITS) - 16 (~16 MiB at 24).
const WBITS: u32 = 24;

/// brotli's maximum meta-block length (MLEN fits in 24 bits).
const MAX_METABLOCK: u64 = 1 << 24;

/// Bits for a simple-code symbol = floor(log2(alphabet_size - 1)) + 1.
const LIT_SYMBOL_BITS: u32 = 8; //  256 literals
const IAC_SYMBOL_BITS: u32 = 10; // 704 insert-and-copy symbols
const DIST_SYMBOL_BITS: u32 = 6; //  64 distance symbols (NPOSTFIX=0, NDIRECT=0)

/// Direct distance code 16 with a single extra bit of 0 decodes to distance 1.
const DIST_CODE_ONE: u32 = 16;

/// Write `<path>` as a brotli stream for `byte` repeated `total` times.
/// Returns the size of the written file in bytes.
pub fn write_file(path: &str, byte: u8, total: u64) -> io::Result<u64> {
    let file = File::create(path)?;
    write_repeat(BufWriter::new(file), byte, total)?;
    Ok(fs::metadata(path)?.len())
}

/// Write a complete brotli stream to `out` (which the caller should buffer).
fn write_repeat<W: Write>(out: W, byte: u8, total: u64) -> io::Result<()> {
    assert!(total > 0, "total must be greater than zero");

    let mut bw = BitWriter::new(out);

    // Stream header: WBITS = 24 is encoded as bit 1 then the 3-bit value 7.
    bw.bits(1, 1)?;
    bw.bits(WBITS - 17, 3)?;

    let mut remaining = total;
    while remaining > 0 {
        let len = remaining.min(MAX_METABLOCK);
        remaining -= len;
        write_metablock(&mut bw, byte, len, remaining == 0)?;
    }

    bw.finish()
}

/// Write one meta-block producing `len` copies of `byte` (1 <= len <= 2^24).
fn write_metablock<W: Write>(
    bw: &mut BitWriter<W>,
    byte: u8,
    len: u64,
    is_last: bool,
) -> io::Result<()> {
    // Meta-block length header.
    bw.bits(is_last as u32, 1)?;
    if is_last {
        bw.bits(0, 1)?; // ISLASTEMPTY = 0
    }
    let nibbles = if len <= (1 << 16) {
        4
    } else if len <= (1 << 20) {
        5
    } else {
        6
    };
    bw.bits(nibbles - 4, 2)?; // MNIBBLES selector
    bw.bits((len - 1) as u32, nibbles * 4)?; // MLEN - 1
    if !is_last {
        bw.bits(0, 1)?; // ISUNCOMPRESSED = 0
    }

    // Compressed-block header: one block type and one prefix tree per category,
    // no distance postfix/direct codes, default literal context mode.
    bw.bits(0, 1)?; // NBLTYPESL = 1
    bw.bits(0, 1)?; // NBLTYPESI = 1
    bw.bits(0, 1)?; // NBLTYPESD = 1
    bw.bits(0, 2)?; // NPOSTFIX = 0
    bw.bits(0, 4)?; // NDIRECT = 0
    bw.bits(0, 2)?; // literal context mode (LSB6, unused with one symbol)
    bw.bits(0, 1)?; // NTREESL = 1
    bw.bits(0, 1)?; // NTREESD = 1

    // Literal tree: the single repeated byte.
    simple_tree(bw, u32::from(byte), LIT_SYMBOL_BITS)?;

    // A copy needs at least 2 bytes, so meta-blocks of 1 or 2 bytes are emitted
    // as literals only, with the copy omitted (the block fills on the insert).
    let (insert_len, copy_len) = if len <= 2 {
        (len as u32, 0)
    } else {
        (1, (len - 1) as u32)
    };
    let ins = insert_code(insert_len);

    if copy_len == 0 {
        simple_tree(bw, combine(ins, 0, true), IAC_SYMBOL_BITS)?;
        simple_tree(bw, DIST_CODE_ONE, DIST_SYMBOL_BITS)?; // defined but unused
        let (extra_bits, base) = INSERT[ins];
        bw.bits(insert_len - base, extra_bits)?; // insert extra; no copy/distance
    } else {
        let cpy = copy_code(copy_len);
        simple_tree(bw, combine(ins, cpy, true), IAC_SYMBOL_BITS)?;
        simple_tree(bw, DIST_CODE_ONE, DIST_SYMBOL_BITS)?;
        let (insert_extra, insert_base) = INSERT[ins];
        bw.bits(insert_len - insert_base, insert_extra)?;
        let (copy_extra, copy_base) = COPY[cpy];
        bw.bits(copy_len - copy_base, copy_extra)?;
        // Literal symbol: 0 bits. Distance symbol 16: 0 bits, then its one
        // extra bit (0) -> distance 1.
        bw.bits(0, 1)?;
    }
    Ok(())
}

/// Emit a simple prefix code holding exactly one `symbol`, which therefore
/// decodes with a zero-length code (RFC 7932 §3.4).
fn simple_tree<W: Write>(bw: &mut BitWriter<W>, symbol: u32, symbol_bits: u32) -> io::Result<()> {
    bw.bits(1, 2)?; // simple prefix code
    bw.bits(0, 2)?; // NSYM - 1 = 0 (one symbol)
    bw.bits(symbol, symbol_bits)?;
    Ok(())
}
