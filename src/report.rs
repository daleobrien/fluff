//! Human-readable reporting of file sizes and ratios.

/// Print one output line: path, compressed size, and compression ratio.
pub fn report(path: &str, original: u64, compressed: u64) {
    let ratio = original as f64 / compressed.max(1) as f64;
    println!(
        "  {path}: {compressed} bytes ({}), {ratio:.0}:1",
        human_bytes(compressed),
    );
}

/// Format a byte count as a human-readable string (binary units).
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_bytes_scales() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1 << 10), "1.0 KiB");
        assert_eq!(human_bytes(1 << 20), "1.0 MiB");
        assert_eq!(human_bytes(1 << 40), "1.0 TiB");
    }
}
