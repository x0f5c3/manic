use manic_lzfse::LzfseRingEncoder;
use std::io;

// Compress stdin into stdout using ring buffers.
fn main() -> io::Result<()> {
    let mut rdr = io::stdin();
    let mut wtr = io::stdout();
    let mut encoder = LzfseRingEncoder::default();
    encoder.encode(&mut rdr, &mut wtr)?;
    Ok(())
}
