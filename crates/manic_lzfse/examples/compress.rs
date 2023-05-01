use manic_lzfse::encode_bytes;
use std::io::{self, Read, Write};

// Compress stdin into stdout using conventional buffers.
fn main() -> io::Result<()> {
    // Read stdin into src.
    let mut rdr = io::stdin();
    let mut src = Vec::default();
    rdr.read_to_end(&mut src)?;

    // Compress src into dst.
    let mut dst = Vec::default();
    encode_bytes(&src, &mut dst)?;

    // Write dst into stdout.
    let mut wtr = io::stdout();
    wtr.write_all(&dst)?;

    Ok(())
}
