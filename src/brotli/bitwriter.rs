//! Buffered, least-significant-bit-first bit sink — the order brotli uses.
//!
//! This mirrors the gzip module's writer but needs only `bits`: every prefix
//! code we emit is a single-symbol (zero-length) code, so there are no
//! multi-bit Huffman codes to write, just raw bit fields.

use std::io::{self, Write};

/// Writes individual bit fields into a byte stream, LSB first.
pub(crate) struct BitWriter<W> {
    inner: W,
    buf: Vec<u8>,
    acc: u32,
    nbits: u32,
}

impl<W: Write> BitWriter<W> {
    pub(crate) fn new(inner: W) -> Self {
        Self {
            inner,
            buf: Vec::with_capacity(1 << 16),
            acc: 0,
            nbits: 0,
        }
    }

    /// Append the low `count` bits of `value`, least-significant bit first.
    pub(crate) fn bits(&mut self, value: u32, count: u32) -> io::Result<()> {
        debug_assert!(count <= 24 && self.nbits < 8);
        self.acc |= (value & mask(count)) << self.nbits;
        self.nbits += count;
        while self.nbits >= 8 {
            self.push(self.acc as u8)?;
            self.acc >>= 8;
            self.nbits -= 8;
        }
        Ok(())
    }

    fn push(&mut self, byte: u8) -> io::Result<()> {
        self.buf.push(byte);
        if self.buf.len() >= (1 << 16) {
            self.inner.write_all(&self.buf)?;
            self.buf.clear();
        }
        Ok(())
    }

    /// Flush the final partial byte (zero-padded) and the underlying writer.
    pub(crate) fn finish(mut self) -> io::Result<()> {
        if self.nbits > 0 {
            self.buf.push(self.acc as u8);
        }
        self.inner.write_all(&self.buf)?;
        self.inner.flush()
    }
}

fn mask(count: u32) -> u32 {
    if count >= 32 {
        u32::MAX
    } else {
        (1u32 << count) - 1
    }
}
