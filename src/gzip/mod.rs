//! A tiny gzip writer specialised for one repeated byte.
//!
//! The input is a single byte repeated `total` times, so there is nothing to
//! "search": we emit the byte once as a literal and then a run of length-258,
//! distance-1 back-references. A hand-built dynamic-Huffman block gives those
//! length and distance symbols one-bit codes, reaching DEFLATE's ~1032:1
//! ceiling — about 1 KB per expanded MiB.
//!
//! Everything here is `std`-only; this module needs no external crate.

mod bitwriter;
mod crc;
mod huffman;

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};

use bitwriter::BitWriter;
use crc::crc32_repeat;
use huffman::{canonical, huffman_lengths, rle_code_lengths};

/// Order in which code-length-alphabet lengths are written (RFC 1951 §3.2.7).
const CL_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Write `<path>` as a gzip stream for `byte` repeated `total` times.
/// Returns the size of the written file in bytes.
pub fn write_file(path: &str, byte: u8, total: u64) -> io::Result<u64> {
    let file = File::create(path)?;
    write_repeat(BufWriter::new(file), byte, total)?;
    Ok(fs::metadata(path)?.len())
}

/// Write a complete gzip stream to `out` (which the caller should buffer).
fn write_repeat<W: Write>(out: W, byte: u8, total: u64) -> io::Result<()> {
    assert!(total > 0, "total must be greater than zero");

    let mut out = out;
    // gzip header: magic, CM=deflate, no flags, mtime=0, XFL=0, OS=unknown.
    out.write_all(&[0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xff])?;

    // Plan the tokens: one leading literal, then as many 258-byte copies as
    // fit, then up to two leftover literals (a copy needs at least 3 bytes).
    let after_first = total - 1;
    let copies = after_first / 258;
    let rem_lits = after_first % 258;
    let literal_count = 1 + rem_lits;

    // Symbol frequencies for the two alphabets.
    let mut litlen_freq = [0u64; 286];
    litlen_freq[byte as usize] += literal_count;
    litlen_freq[256] += 1; // end-of-block
    if copies > 0 {
        litlen_freq[285] += copies; // length 258
    }
    let mut dist_freq = [0u64; 1];
    dist_freq[0] += copies; // distance 1

    // Code lengths, then canonical codes, for both alphabets.
    let litlen_len = huffman_lengths(&litlen_freq);
    let mut dist_len = huffman_lengths(&dist_freq);
    if dist_len[0] == 0 {
        dist_len[0] = 1; // a block must define at least one distance code
    }
    let litlen_code = canonical(&litlen_len);
    let dist_code = canonical(&dist_len);

    // Run-length-encode the concatenated code lengths for the block header.
    let mut combined = Vec::with_capacity(litlen_len.len() + dist_len.len());
    combined.extend_from_slice(&litlen_len);
    combined.extend_from_slice(&dist_len);
    let rle = rle_code_lengths(&combined);

    // Huffman code for the code-length alphabet itself.
    let mut cl_freq = [0u64; 19];
    for &(sym, _, _) in &rle {
        cl_freq[sym as usize] += 1;
    }
    let cl_len = huffman_lengths(&cl_freq);
    let cl_code = canonical(&cl_len);

    let mut bw = BitWriter::new(out);
    bw.bits(1, 1)?; // BFINAL = 1 (final block)
    bw.bits(2, 2)?; // BTYPE = 10 (dynamic Huffman)
    bw.bits(29, 5)?; // HLIT  = 286 - 257
    bw.bits(0, 5)?; // HDIST =   1 -   1
    bw.bits(15, 4)?; // HCLEN =  19 -   4 (all 19 lengths sent)
    for &idx in &CL_ORDER {
        bw.bits(u32::from(cl_len[idx]), 3)?;
    }
    for &(sym, extra, extra_bits) in &rle {
        bw.code(cl_code[sym as usize], cl_len[sym as usize])?;
        if extra_bits > 0 {
            bw.bits(extra, extra_bits)?;
        }
    }

    // The data: leading literal, the copies, leftover literals, end-of-block.
    let byte = byte as usize;
    bw.code(litlen_code[byte], litlen_len[byte])?;
    for _ in 0..copies {
        bw.code(litlen_code[285], litlen_len[285])?;
        bw.code(dist_code[0], dist_len[0])?;
    }
    for _ in 0..rem_lits {
        bw.code(litlen_code[byte], litlen_len[byte])?;
    }
    bw.code(litlen_code[256], litlen_len[256])?;

    let mut out = bw.finish()?;

    // gzip trailer: CRC-32 of the data, then the size mod 2^32, little-endian.
    let crc = crc32_repeat(byte as u8, total);
    out.write_all(&crc.to_le_bytes())?;
    out.write_all(&((total & 0xffff_ffff) as u32).to_le_bytes())?;
    out.flush()
}
