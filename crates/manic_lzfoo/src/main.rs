use clap::{crate_version, App, AppSettings, Arg, ArgMatches, SubCommand};
use manic_lzfse::{LzfseRingDecoder, LzfseRingEncoder};

use core::panic;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::process;
use std::time::Instant;

const STDIN: &str = "stdin";
const STDOUT: &str = "stdout";

#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode {
    Encode,
    Decode,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Encode => f.write_str("encode"),
            Mode::Decode => f.write_str("decode"),
        }
    }
}

fn main() {
    process::exit(match execute() {
        Ok(()) => 0,
        Err(manic_lzfse::Error::Io(err)) if err.kind() == io::ErrorKind::BrokenPipe => 0,
        Err(manic_lzfse::Error::Io(err)) => {
            eprint!("Error: IO: {}", err);
            1
        }
        Err(manic_lzfse::Error::BufferOverflow) => {
            eprint!("Error: Buffer overflow");
            1
        }
        Err(err) => {
            eprintln!("Error: Decode: {}", err);
            1
        }
    });
}

fn execute() -> manic_lzfse::Result<()> {
    let matches = arg_matches();
    match matches.subcommand() {
        ("-encode", Some(m)) => {
            let input = m.value_of("input");
            let output = m.value_of("output");
            let verbose = m.occurrences_of("v") != 0;
            match (input, output) {
                (None, None) => encode(io::stdin(), io::stdout(), STDIN, STDOUT, verbose),
                (Some(r), None) => encode(File::open(r)?, io::stdout(), r, STDOUT, verbose),
                (None, Some(w)) => encode(io::stdin(), File::create(w)?, STDIN, w, verbose),
                (Some(r), Some(w)) => encode(File::open(r)?, File::create(w)?, r, w, verbose),
            }?;
        }
        ("-decode", Some(m)) => {
            let input = m.value_of("input");
            let output = m.value_of("output");
            let verbose = m.occurrences_of("v") != 0;
            match (input, output) {
                (None, None) => decode(io::stdin(), io::stdout(), STDIN, STDOUT, verbose),
                (Some(r), None) => decode(File::open(r)?, io::stdout(), r, STDOUT, verbose),
                (None, Some(w)) => decode(io::stdin(), File::create(w)?, STDIN, w, verbose),
                (Some(r), Some(w)) => decode(File::open(r)?, File::create(w)?, r, w, verbose),
            }?;
        }
        _ => panic!(),
    };

    Ok(())
}

#[inline(never)]
fn encode<R: Read, W: Write>(
    mut src: R,
    mut dst: W,
    input: &str,
    output: &str,
    verbose: bool,
) -> io::Result<()> {
    let instant = if verbose { Some(Instant::now()) } else { None };
    let (n_raw_bytes, n_payload_bytes) = LzfseRingEncoder::default().encode(&mut src, &mut dst)?;
    if let Some(start) = instant {
        stats(
            start,
            n_raw_bytes,
            n_payload_bytes,
            input,
            output,
            Mode::Encode,
        )
    }
    Ok(())
}

fn decode<R: Read, W: Write>(
    mut src: R,
    mut dst: W,
    input: &str,
    output: &str,
    verbose: bool,
) -> manic_lzfse::Result<()> {
    let instant = if verbose { Some(Instant::now()) } else { None };
    let (n_raw_bytes, n_payload_bytes) = LzfseRingDecoder::default().decode(&mut src, &mut dst)?;
    if let Some(start) = instant {
        stats(
            start,
            n_raw_bytes,
            n_payload_bytes,
            input,
            output,
            Mode::Decode,
        )
    }
    Ok(())
}

#[cold]
fn stats(
    start: Instant,
    n_input_bytes: u64,
    n_output_bytes: u64,
    input: &str,
    output: &str,
    mode: Mode,
) {
    let duration = Instant::now() - start;
    let secs = duration.as_secs_f64();
    let (n_raw_bytes, n_payload_bytes) = match mode {
        Mode::Encode => (n_input_bytes, n_output_bytes),
        Mode::Decode => (n_output_bytes, n_input_bytes),
    };
    let ns_per_byte = 1.0e9 * secs / n_raw_bytes as f64;
    let mb_per_sec = n_raw_bytes as f64 / secs / 1024.0 / 1024.0;
    if output == STDOUT {
        eprintln!();
    }
    eprintln!("LZFSE {}", mode);
    eprintln!("Input: {}", input);
    eprintln!("Output: {}", output);
    eprintln!("Input size: {} B", n_input_bytes);
    eprintln!("Output size: {} B", n_output_bytes);
    eprintln!(
        "Compression ratio: {:.3}",
        n_raw_bytes as f64 / n_payload_bytes as f64
    );
    eprintln!("Speed: {:.2} ns/B, {:.2} MB/s", ns_per_byte, mb_per_sec);
}

fn arg_matches() -> ArgMatches<'static> {
    App::new("lzfoo")
        .version(crate_version!())
        .author("Vin Singh <github.com/shampoofactory>")
        .about("LZFSE compressor/ decompressor")
        .after_help("See 'lzfoo help <command>' for more information on a specific command.")
        .subcommand(
            SubCommand::with_name("-decode")
                .alias("decode")
                .about("Decode (decompress)")
                .after_help(
                    "If no input/ output specified reads/ writes from standard input/ output.",
                )
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .help("input")
                        .takes_value(true)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .help("output")
                        .takes_value(true)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::with_name("v")
                        .short("v")
                        .help("Sets the level of verbosity"),
                ),
        )
        .subcommand(
            SubCommand::with_name("-encode")
                .alias("encode")
                .about("Encode (compress)")
                .after_help(
                    "If no input/ output specified reads/ writes from standard input/ output",
                )
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .help("input")
                        .takes_value(true)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .help("output")
                        .takes_value(true)
                        .value_name("FILE"),
                )
                .arg(
                    Arg::with_name("v")
                        .short("v")
                        .help("Sets the level of verbosity"),
                ),
        )
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .get_matches()
}
