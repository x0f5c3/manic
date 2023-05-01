use manic_lzfse::decode_bytes;
use std::io::{self, Read, Write};

// Decompress stdin into stdout using conventional buffers.
fn main() -> io::Result<()> {
    // Read stdin into src.
    let mut rdr = io::stdin();
    let mut src = Vec::default();
    rdr.read_to_end(&mut src)?;

    // Decompress src into dst.
    let mut dst = Vec::default();
    decode_bytes(&src, &mut dst)?;

    // Write dst into stdout.
    let mut wtr = io::stdout();
    wtr.write_all(&dst)?;

    Ok(())
}
