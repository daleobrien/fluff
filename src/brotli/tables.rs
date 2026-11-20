//! Insert/copy length codes and the insert-and-copy command mapping (RFC 7932).

/// Insert-length codes: `(extra_bits, base)` (RFC 7932 §5).
#[rustfmt::skip]
pub(crate) const INSERT: [(u32, u32); 24] = [
    (0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5), (1, 6), (1, 8),
    (2, 10), (2, 14), (3, 18), (3, 26), (4, 34), (4, 50), (5, 66), (5, 98),
    (6, 130), (7, 194), (8, 322), (9, 578), (10, 1090), (12, 2114), (14, 6210), (24, 22594),
];

/// Copy-length codes: `(extra_bits, base)` (RFC 7932 §5). Minimum copy is 2.
#[rustfmt::skip]
pub(crate) const COPY: [(u32, u32); 24] = [
    (0, 2), (0, 3), (0, 4), (0, 5), (0, 6), (0, 7), (0, 8), (0, 9),
    (1, 10), (1, 12), (2, 14), (2, 18), (3, 22), (3, 30), (4, 38), (4, 54),
    (5, 70), (5, 102), (6, 134), (7, 198), (8, 326), (9, 582), (10, 1094), (24, 2118),
];

/// The length code whose range contains `len` (the one with the largest base
/// not exceeding `len`). Callers guarantee `len` is in range for the table.
pub(crate) fn insert_code(len: u32) -> usize {
    INSERT
        .iter()
        .rposition(|&(_, base)| len >= base)
        .expect("insert code 0 has base 0, so any length matches")
}

pub(crate) fn copy_code(len: u32) -> usize {
    COPY.iter()
        .rposition(|&(_, base)| len >= base)
        .expect("copy_code is only called with len >= 2")
}

/// Each 64-symbol block of the insert-and-copy alphabet, as
/// `(insert_code_offset, copy_code_offset, implicit_distance)` (RFC 7932 §5).
const BLOCKS: [(u32, u32, bool); 11] = [
    (0, 0, true),
    (0, 8, true),
    (0, 0, false),
    (0, 8, false),
    (8, 0, false),
    (8, 8, false),
    (0, 16, false),
    (16, 0, false),
    (8, 16, false),
    (16, 8, false),
    (16, 16, false),
];

/// Combine an insert-length code and a copy-length code into an
/// insert-and-copy command symbol (0..=703). With `explicit` true the command
/// carries its own distance; otherwise the last distance is reused implicitly.
pub(crate) fn combine(ins_code: usize, copy_code: usize, explicit: bool) -> u32 {
    let ins = ins_code as u32;
    let copy = copy_code as u32;
    let (ins_off, ins_sub) = ((ins / 8) * 8, ins % 8);
    let (copy_off, copy_sub) = ((copy / 8) * 8, copy % 8);
    let block = BLOCKS
        .iter()
        .position(|&(io, co, implicit)| io == ins_off && co == copy_off && implicit == !explicit)
        .expect("every insert/copy offset pair maps to exactly one command block")
        as u32;
    block * 64 + (ins_sub << 3) + copy_sub
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_codes_pick_the_enclosing_range() {
        assert_eq!(insert_code(0), 0);
        assert_eq!(insert_code(1), 1);
        assert_eq!(insert_code(7), 6); // base 6, +1 extra
        assert_eq!(copy_code(2), 0);
        assert_eq!(copy_code(258), 19); // base 198 .. 325
        assert_eq!(copy_code((1 << 24) - 1), 23);
    }

    #[test]
    fn combine_matches_known_symbols() {
        // insert 1, copy 5, explicit distance -> block 2 (0x80) + (1<<3) + 5.
        assert_eq!(combine(1, 5, true), 128 + 8 + 5);
        // insert 0, copy 0, implicit distance -> symbol 0.
        assert_eq!(combine(0, 0, false), 0);
    }
}
