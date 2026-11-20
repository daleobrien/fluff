//! Huffman code construction and code-length run-length encoding (RFC 1951).

/// Optimal Huffman code lengths for the given symbol frequencies. Symbols with
/// zero frequency get length 0; a lone symbol gets length 1.
pub(crate) fn huffman_lengths(freqs: &[u64]) -> Vec<u8> {
    let mut lengths = vec![0u8; freqs.len()];
    let mut nodes: Vec<(u64, Vec<usize>)> = freqs
        .iter()
        .enumerate()
        .filter(|&(_, &f)| f > 0)
        .map(|(i, &f)| (f, vec![i]))
        .collect();

    match nodes.len() {
        0 => return lengths,
        1 => {
            lengths[nodes[0].1[0]] = 1;
            return lengths;
        }
        _ => {}
    }

    while nodes.len() > 1 {
        // Combine the two lowest-frequency nodes; descending sort so the
        // smallest two are at the end and cheap to pop.
        nodes.sort_unstable_by_key(|n| core::cmp::Reverse(n.0));
        let (fa, sa) = nodes.pop().unwrap();
        let (fb, sb) = nodes.pop().unwrap();
        for &s in sa.iter().chain(&sb) {
            lengths[s] += 1;
        }
        let mut merged = sa;
        merged.extend(sb);
        nodes.push((fa + fb, merged));
    }
    lengths
}

/// Canonical Huffman codes for the given code lengths (RFC 1951 §3.2.2).
pub(crate) fn canonical(lengths: &[u8]) -> Vec<u16> {
    let max = lengths.iter().copied().max().unwrap_or(0);
    let mut codes = vec![0u16; lengths.len()];
    if max == 0 {
        return codes;
    }

    let mut bl_count = vec![0u16; usize::from(max) + 1];
    for &l in lengths {
        if l > 0 {
            bl_count[usize::from(l)] += 1;
        }
    }

    let mut next = vec![0u16; usize::from(max) + 1];
    let mut code = 0u16;
    for bits in 1..=usize::from(max) {
        code = (code + bl_count[bits - 1]) << 1;
        next[bits] = code;
    }

    for (i, &l) in lengths.iter().enumerate() {
        if l > 0 {
            codes[i] = next[usize::from(l)];
            next[usize::from(l)] += 1;
        }
    }
    codes
}

/// Run-length-encode code lengths into code-length-alphabet symbols.
/// Each item is `(symbol, extra_value, extra_bit_count)`.
pub(crate) fn rle_code_lengths(lengths: &[u8]) -> Vec<(u8, u32, u32)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lengths.len() {
        let value = lengths[i];
        let mut count = 1;
        while i + count < lengths.len() && lengths[i + count] == value {
            count += 1;
        }
        i += count;

        if value == 0 {
            while count >= 11 {
                let take = count.min(138);
                out.push((18u8, (take - 11) as u32, 7u32));
                count -= take;
            }
            while count >= 3 {
                let take = count.min(10);
                out.push((17u8, (take - 3) as u32, 3u32));
                count -= take;
            }
            for _ in 0..count {
                out.push((0u8, 0, 0));
            }
        } else {
            out.push((value, 0, 0));
            count -= 1;
            while count >= 3 {
                let take = count.min(6);
                out.push((16u8, (take - 3) as u32, 2u32));
                count -= take;
            }
            for _ in 0..count {
                out.push((value, 0, 0));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_single() {
        assert_eq!(huffman_lengths(&[0, 0, 0]), vec![0, 0, 0]);
        assert_eq!(huffman_lengths(&[0, 7, 0]), vec![0, 1, 0]);
    }

    #[test]
    fn lengths_satisfy_kraft() {
        // A skewed three-symbol alphabet -> lengths {1,2,2}.
        let lens = huffman_lengths(&[4064, 150, 1]);
        let kraft: f64 = lens
            .iter()
            .filter(|&&l| l > 0)
            .map(|&l| 2f64.powi(-i32::from(l)))
            .sum();
        assert!((kraft - 1.0).abs() < 1e-9, "lengths {lens:?} not complete");
    }

    #[test]
    fn canonical_codes_are_prefix_free() {
        // lengths [2,1,3,3] -> a valid complete code.
        let lens = [2u8, 1, 3, 3];
        let codes = canonical(&lens);
        // Check no code is a prefix of another.
        for a in 0..lens.len() {
            for b in 0..lens.len() {
                if a == b || lens[a] == 0 || lens[b] == 0 || lens[a] > lens[b] {
                    continue;
                }
                let shift = lens[b] - lens[a];
                assert_ne!(codes[b] >> shift, codes[a], "code {a} is a prefix of {b}");
            }
        }
    }

    #[test]
    fn rle_collapses_long_zero_runs() {
        // 200 zeros should become a couple of symbol-18 runs, not 200 entries.
        let lengths = vec![0u8; 200];
        let rle = rle_code_lengths(&lengths);
        assert!(rle.len() <= 2);
        assert!(rle.iter().all(|&(sym, _, _)| sym == 18));
    }

    #[test]
    fn rle_emits_literal_then_repeat() {
        let mut lengths = vec![2u8; 5];
        lengths.push(1);
        let rle = rle_code_lengths(&lengths);
        // first a literal 2, then a copy-previous (16), then the trailing 1.
        assert_eq!(rle[0], (2, 0, 0));
        assert_eq!(rle[1].0, 16);
        assert_eq!(rle.last().unwrap(), &(1, 0, 0));
    }
}
