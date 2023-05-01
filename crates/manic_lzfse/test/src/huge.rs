use test_kit::{Rng, Seq};

use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

// We want the debug build for it's tighter internal state validation.
const LZFOO: &str = "../target/debug/lzfoo";

// Total number of bytes. Theoretical max: `i64::MAX as u64`, limited by RingLzWriter.
const N_BYTES: u64 = 0x0000_0010_0000_0000; // 64 GB

// Masked sequence to reduce entropy and increase availability of matches. We want to stress
// the compressor internals.
const S_MASK: u32 = 0x0303_0000;

// Huge file test.
#[test]
fn pipe_64_gb() -> io::Result<()> {
    // Concurrent 64 GB pipe test: seq gen > lzfoo encode > lzfoo decode > seq test
    //
    // We use identical Seq instances to limit memory requirements, the test should take no more
    // than a few MB of memory.

    // Encoder
    let mut enc = Command::new(LZFOO)
        .arg("-encode")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start lzfoo encode process");
    let mut enci = enc.stdin.take().expect("failed to open lzfoo encode stdin");
    let enco = enc.stdout.expect("failed to open lzfoo encode stdout");

    // Decoder
    let dec = Command::new(LZFOO)
        .arg("-decode")
        .stdin(Stdio::from(enco))
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start lzfoo decode process");
    let mut deco = dec.stdout.expect("failed to open lzfoo decode stdout");

    // Pipe in Seq.
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut seqi = Seq::masked(Rng::default(), S_MASK);
        let mut buf = vec![0u8; 0x4000];
        let mut n = N_BYTES;
        loop {
            let m = (buf.len() as u64).min(n) as usize;
            n -= m as u64;
            if m == 0 {
                break;
            }
            seqi.read_exact(&mut buf[..m]).unwrap();
            enci.write_all(&buf[..m]).expect("thread io error");
        }
        tx.send(seqi).expect("tx error");
    });

    // Pipe out and validate.
    let mut seqo = Seq::masked(Rng::default(), S_MASK);
    let mut buf = vec![0u8; 0x4000];
    let mut n_bytes = 0;
    loop {
        let n = deco.read(&mut buf)?;
        n_bytes += n as u64;
        // Write to the ether, however an error will be thrown if bytes do not match seq.
        seqo.write_all(&buf[..n])?;
        if n == 0 {
            break;
        }
    }
    let seqi = rx.recv().expect("rx error");

    // Check byte length and Seq match.
    assert_eq!(n_bytes, N_BYTES);
    assert_eq!(seqo, seqi);

    Ok(())
}
