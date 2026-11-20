//! CRC-32 (gzip/zlib) for a repeated byte, via CRC combination in O(log n).

/// CRC-32 polynomial (reversed), as used by gzip/zlib.
const POLY: u32 = 0xEDB8_8320;

/// CRC-32 of `total` copies of `byte`.
pub(crate) fn crc32_repeat(byte: u8, total: u64) -> u32 {
    let single = crc32_bytes(&[byte]);
    let mut result = 0u32; // CRC of the empty string
    let mut block_crc = single;
    let mut block_len = 1u64;
    let mut n = total;
    while n > 0 {
        if n & 1 == 1 {
            result = combine(result, block_crc, block_len);
        }
        block_crc = combine(block_crc, block_crc, block_len);
        block_len = block_len.wrapping_mul(2);
        n >>= 1;
    }
    result
}

fn crc32_bytes(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ POLY
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Apply a GF(2) operator matrix to a vector.
fn gf2_times(mat: &[u32; 32], mut vec: u32) -> u32 {
    let mut sum = 0u32;
    let mut i = 0;
    while vec != 0 {
        if vec & 1 == 1 {
            sum ^= mat[i];
        }
        vec >>= 1;
        i += 1;
    }
    sum
}

fn gf2_square(dst: &mut [u32; 32], src: &[u32; 32]) {
    for n in 0..32 {
        dst[n] = gf2_times(src, src[n]);
    }
}

/// Combine two CRC-32 values: CRC(A ++ B) from CRC(A), CRC(B), len(B).
/// Mirrors zlib's `crc32_combine`.
fn combine(crc1: u32, crc2: u32, mut len2: u64) -> u32 {
    if len2 == 0 {
        return crc1;
    }

    let mut even = [0u32; 32];
    let mut odd = [0u32; 32];

    // Operator for one zero bit.
    odd[0] = POLY;
    let mut row = 1u32;
    for slot in odd.iter_mut().skip(1) {
        *slot = row;
        row <<= 1;
    }

    gf2_square(&mut even, &odd); // two zero bits
    gf2_square(&mut odd, &even); // four zero bits

    let mut crc = crc1;
    loop {
        gf2_square(&mut even, &odd); // one zero byte, then quadrupling each step
        if len2 & 1 == 1 {
            crc = gf2_times(&even, crc);
        }
        len2 >>= 1;
        if len2 == 0 {
            break;
        }
        gf2_square(&mut odd, &even);
        if len2 & 1 == 1 {
            crc = gf2_times(&odd, crc);
        }
        len2 >>= 1;
        if len2 == 0 {
            break;
        }
    }
    crc ^ crc2
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference CRC-32 over an actually-materialised buffer.
    fn reference(byte: u8, n: usize) -> u32 {
        crc32_bytes(&vec![byte; n])
    }

    #[test]
    fn matches_reference_for_small_sizes() {
        for &byte in &[b'a', 0u8, 0xff] {
            for n in [1usize, 2, 3, 255, 256, 257, 1000, 65536] {
                assert_eq!(
                    crc32_repeat(byte, n as u64),
                    reference(byte, n),
                    "byte={byte} n={n}"
                );
            }
        }
    }

    #[test]
    fn known_vector() {
        // CRC-32 of "aaaa".
        assert_eq!(crc32_repeat(b'a', 4), 0xAD98_E545);
    }
}
