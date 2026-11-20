//! fluff — write tiny gzip and brotli files that expand to one repeated byte.
//!
//! One byte, puffed up into megabytes of nothing. Handy for testing
//! decompression limits, streaming buffers, or just seeing how far a
//! compressor will go on maximally boring input.
//!
//! Both the gzip and brotli streams are hand-written, with no compression
//! library (see [`gzip`] and [`brotli`]).

mod brotli;
mod cli;
mod gzip;
mod report;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::Cli;
use crate::report::{human_bytes, report};

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> std::io::Result<()> {
    let gz_path = format!("{}.gz", cli.out);
    let br_path = format!("{}.br", cli.out);

    let gz_size = gzip::write_file(&gz_path, cli.byte, cli.size)?;
    let br_size = brotli::write_file(&br_path, cli.byte, cli.size)?;

    println!(
        "{} bytes ({}) of {:?} repeated:",
        cli.size,
        human_bytes(cli.size),
        char::from(cli.byte),
    );
    report(&gz_path, cli.size, gz_size);
    report(&br_path, cli.size, br_size);
    Ok(())
}
