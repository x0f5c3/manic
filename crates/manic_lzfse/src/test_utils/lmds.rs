use crate::encode::Backend;
use crate::lmd::{self, Lmd, LmdMax, MatchDistance};

use std::io;

/// Check that `literals` and `lmds` define `bytes`.
pub fn check_lmds<T: LmdMax>(bytes: &[u8], mut literals: &[u8], lmds: &[Lmd<T>]) -> bool {
    let mut i = 0;
    for &Lmd(literal_len, match_len, match_distance) in lmds {
        let literal_len = literal_len.get() as usize;
        let match_len = match_len.get() as usize;
        let match_distance = match_distance.get() as usize;
        // Check literals.
        if literals.len() < literal_len || bytes.len() < i + literal_len {
            return false;
        }
        if literals[..literal_len] != bytes[i..i + literal_len] {
            return false;
        }
        i += literal_len;
        literals = &literals[literal_len..];
        // Check match.
        if bytes.len() < i + match_len {
            return false;
        }
        if bytes[i - match_distance..i - match_distance + match_len] != bytes[i..i + match_len] {
            return false;
        }
        i += match_len;
    }
    literals.is_empty() && i == bytes.len()
}

/// Encode `literals` and `lmds` into cleared `dst` using `backend` returning
/// (n_raw_bytes, n_payload_bytes).
pub fn encode_lmds<B: Backend, T: LmdMax>(
    dst: &mut Vec<u8>,
    backend: &mut B,
    mut literals: &[u8],
    lmds: &[Lmd<T>],
) -> io::Result<(u32, u32)> {
    let len = lmd::n_raw_bytes(lmds);
    dst.clear();
    dst.reserve(len as usize);
    backend.init(dst, Some(len))?;
    for &Lmd(literal_len, match_len, match_distance) in lmds {
        let literal_len = literal_len.get() as usize;
        let match_len = match_len.get();
        let match_distance = match_distance.get();
        if match_len == 0 {
            backend.push_literals(dst, &literals[..literal_len])?;
        } else {
            let match_distance = MatchDistance::new(match_distance);
            backend.push_match(dst, &literals[..literal_len], match_len, match_distance)?;
        }
        literals = &literals[literal_len..];
    }
    backend.finalize(dst)?;
    assert_eq!(literals.len(), 0);
    Ok((len, dst.len() as u32))
}
