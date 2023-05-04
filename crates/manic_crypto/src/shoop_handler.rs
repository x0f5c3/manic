use byteorder::{LittleEndian, WriteBytesExt};

use chacha20poly1305::aead::{AeadCore, AeadInPlace};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};

const MAX_NONCE: u64 = u64::MAX;
