//! Command-line interface (clap) and value parsers.

use clap::{Parser, ValueEnum};

/// Write tiny gzip and brotli files that expand to one repeated byte.
#[derive(Parser, Debug)]
#[command(
    version,
    about,
    after_help = "gzip (DEFLATE) caps out near 1032:1, so ~1 MiB of one byte is about the \
                  smallest input that still yields a ~1 KB .gz. Brotli's large window goes \
                  much further."
)]
pub struct Cli {
    /// ASCII character to repeat
    #[arg(short = 'c', long = "char", value_name = "CHAR", default_value = "a", value_parser = parse_char)]
    pub byte: u8,

    /// Decompressed size; suffixes k/m/g/t = KiB/MiB/GiB/TiB
    #[arg(short, long, default_value = "1m", value_parser = parse_size)]
    pub size: u64,

    /// Output prefix; writes <PREFIX>.gz and/or <PREFIX>.br
    #[arg(short, long, value_name = "PREFIX", default_value = "repeat")]
    pub out: String,

    /// Which file(s) to write
    #[arg(short, long, value_enum, default_value_t = Format::Both)]
    pub format: Format,
}

/// Which compressed file(s) `fluff` should write.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Write both the gzip and brotli files.
    Both,
    /// Write only the gzip `.gz` file.
    Gz,
    /// Write only the brotli `.br` file.
    Br,
}

impl Format {
    /// Whether the gzip `.gz` file should be written.
    pub fn gzip(self) -> bool {
        matches!(self, Format::Both | Format::Gz)
    }

    /// Whether the brotli `.br` file should be written.
    pub fn brotli(self) -> bool {
        matches!(self, Format::Both | Format::Br)
    }
}

/// Accept exactly one ASCII character and return its byte value.
fn parse_char(value: &str) -> Result<u8, String> {
    let mut chars = value.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii() => Ok(c as u8),
        (Some(_), None) => Err("must be a single ASCII character".to_string()),
        _ => Err("needs exactly one character".to_string()),
    }
}

/// Parse a non-zero byte count with an optional binary suffix: k/m/g/t.
fn parse_size(value: &str) -> Result<u64, String> {
    let trimmed = value.trim();
    let split = trimmed
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(trimmed.len());
    let (digits, suffix) = trimmed.split_at(split);

    let number: u64 = digits
        .parse()
        .map_err(|_| format!("invalid size `{value}`"))?;
    let multiplier: u64 = match suffix.to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" | "kib" => 1 << 10,
        "m" | "mb" | "mib" => 1 << 20,
        "g" | "gb" | "gib" => 1 << 30,
        "t" | "tb" | "tib" => 1 << 40,
        other => return Err(format!("unknown size suffix `{other}`")),
    };

    match number.checked_mul(multiplier) {
        Some(0) => Err("size must be greater than zero".to_string()),
        Some(total) => Ok(total),
        None => Err(format!("size `{value}` is too large")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn size_plain_and_suffixes() {
        assert_eq!(parse_size("1024"), Ok(1024));
        assert_eq!(parse_size("1k"), Ok(1 << 10));
        assert_eq!(parse_size("2M"), Ok(2 << 20));
        assert_eq!(parse_size("1gib"), Ok(1 << 30));
        assert_eq!(parse_size("1t"), Ok(1 << 40));
        assert_eq!(parse_size("2TiB"), Ok(2 << 40));
        assert_eq!(parse_size("  4kb "), Ok(4 << 10));
    }

    #[test]
    fn size_rejects_junk() {
        assert!(parse_size("").is_err());
        assert!(parse_size("0").is_err());
        assert!(parse_size("abc").is_err());
        assert!(parse_size("12x").is_err());
        assert!(parse_size("99999999999999999999g").is_err()); // overflow
        assert!(parse_size("16777216t").is_err()); // 2^64 bytes overflows u64
    }

    #[test]
    fn char_is_single_ascii() {
        assert_eq!(parse_char("a"), Ok(b'a'));
        assert_eq!(parse_char("0"), Ok(b'0'));
        assert!(parse_char("ab").is_err());
        assert!(parse_char("").is_err());
        assert!(parse_char("é").is_err());
    }

    #[test]
    fn args_defaults_and_overrides() {
        let cli = Cli::try_parse_from(["fluff"]).unwrap();
        assert_eq!(
            (cli.byte, cli.size, cli.out.as_str()),
            (b'a', 1 << 20, "repeat")
        );

        let cli = Cli::try_parse_from(["fluff", "-c", "z", "-s", "8k", "-o", "big"]).unwrap();
        assert_eq!(
            (cli.byte, cli.size, cli.out.as_str()),
            (b'z', 8 << 10, "big")
        );
    }

    #[test]
    fn format_selects_outputs() {
        let cli = Cli::try_parse_from(["fluff"]).unwrap();
        assert_eq!(cli.format, Format::Both);
        assert!(cli.format.gzip() && cli.format.brotli());

        let cli = Cli::try_parse_from(["fluff", "--format", "br"]).unwrap();
        assert!(cli.format.brotli() && !cli.format.gzip());

        let cli = Cli::try_parse_from(["fluff", "-f", "gz"]).unwrap();
        assert!(cli.format.gzip() && !cli.format.brotli());

        assert!(Cli::try_parse_from(["fluff", "-f", "zip"]).is_err());
    }

    #[test]
    fn args_reject_bad_input() {
        assert!(Cli::try_parse_from(["fluff", "-s", "0"]).is_err());
        assert!(Cli::try_parse_from(["fluff", "--nope"]).is_err());
        assert!(Cli::try_parse_from(["fluff", "-c"]).is_err()); // missing value
    }
}
