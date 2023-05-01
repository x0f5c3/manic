use manic_lzfse::LzfseRingDecoder;
use std::io;

// Decompress stdin into stdout using ring buffers.
fn main() -> io::Result<()> {
    let mut rdr = io::stdin();
    let mut wtr = io::stdout();
    let mut decoder = LzfseRingDecoder::default();
    decoder.decode(&mut rdr, &mut wtr)?;
    Ok(())
}
