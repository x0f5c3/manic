const CORPUS_HTML: &[u8] = include_bytes!("../../data/snappy/html.lzfse");
const CORPUS_URLS_10K: &[u8] = include_bytes!("../../data/snappy/urls.10K.lzfse");
const CORPUS_FIREWORKS: &[u8] = include_bytes!("../../data/snappy/fireworks.jpeg.lzfse");
const CORPUS_PAPER_100K: &[u8] = include_bytes!("../../data/snappy/paper-100k.pdf.lzfse");
const CORPUS_HTML_X_4: &[u8] = include_bytes!("../../data/snappy/html_x_4.lzfse");
const CORPUS_ALICE29: &[u8] = include_bytes!("../../data/snappy/alice29.txt.lzfse");
const CORPUS_ASYOULIK: &[u8] = include_bytes!("../../data/snappy/asyoulik.txt.lzfse");
const CORPUS_LCET10: &[u8] = include_bytes!("../../data/snappy/lcet10.txt.lzfse");
const CORPUS_PLRABN12: &[u8] = include_bytes!("../../data/snappy/plrabn12.txt.lzfse");
const CORPUS_GEOPROTO: &[u8] = include_bytes!("../../data/snappy/geo.protodata.lzfse");
const CORPUS_KPPKN: &[u8] = include_bytes!("../../data/snappy/kppkn.gtb.lzfse");

const CORPUS_HTML_HASH: &[u8] = include_bytes!("../../data/snappy/html.hash");
const CORPUS_URLS_10K_HASH: &[u8] = include_bytes!("../../data/snappy/urls.10K.hash");
const CORPUS_FIREWORKS_HASH: &[u8] = include_bytes!("../../data/snappy/fireworks.jpeg.hash");
const CORPUS_PAPER_100K_HASH: &[u8] = include_bytes!("../../data/snappy/paper-100k.pdf.hash");
const CORPUS_HTML_X_4_HASH: &[u8] = include_bytes!("../../data/snappy/html_x_4.hash");
const CORPUS_ALICE29_HASH: &[u8] = include_bytes!("../../data/snappy/alice29.txt.hash");
const CORPUS_ASYOULIK_HASH: &[u8] = include_bytes!("../../data/snappy/asyoulik.txt.hash");
const CORPUS_LCET10_HASH: &[u8] = include_bytes!("../../data/snappy/lcet10.txt.hash");
const CORPUS_PLRABN12_HASH: &[u8] = include_bytes!("../../data/snappy/plrabn12.txt.hash");
const CORPUS_GEOPROTO_HASH: &[u8] = include_bytes!("../../data/snappy/geo.protodata.hash");
const CORPUS_KPPKN_HASH: &[u8] = include_bytes!("../../data/snappy/kppkn.gtb.hash");

const CORPUS_COMPOUND: &[u8] = include_bytes!("../../data/special/compound.lzfse");
const CORPUS_COMPOUND_HASH: &[u8] = include_bytes!("../../data/special/compound.hash");

#[cfg(feature = "large_data")]
const CORPUS_ENWIK8: &[u8] = include_bytes!("../../data/large/enwik8.lzfse");
#[cfg(feature = "large_data")]
const CORPUS_ENWIK8_HASH: &[u8] = include_bytes!("../../data/large/enwik8.hash");

macro_rules! test_codec {
    ($name:ident, $data:ident, $hash:ident) => {
        mod $name {
            use crate::buddy::Buddy;
            use crate::ops;

            use std::io;

            #[test]
            pub fn decode() -> io::Result<()> {
                Buddy::default().decode_hash(super::$data, super::$hash, ops::decode)
            }

            #[test]
            pub fn decode_bytes() -> io::Result<()> {
                Buddy::default().decode_hash(super::$data, super::$hash, ops::decode_bytes)
            }

            #[test]
            pub fn decode_reader() -> io::Result<()> {
                Buddy::default().decode_hash(super::$data, super::$hash, ops::decode_reader)
            }

            #[test]
            pub fn decode_reader_bytes() -> io::Result<()> {
                Buddy::default().decode_hash(super::$data, super::$hash, ops::decode_reader_bytes)
            }

            #[test]
            pub fn encode() -> io::Result<()> {
                Buddy::default().decode_encode_decode(super::$data, ops::encode)
            }

            #[test]
            pub fn encode_bytes() -> io::Result<()> {
                Buddy::default().decode_encode_decode(super::$data, ops::encode_bytes)
            }

            #[test]
            pub fn encode_writer() -> io::Result<()> {
                Buddy::default().decode_encode_decode(super::$data, ops::encode_writer)
            }

            #[test]
            pub fn encode_writer_bytes() -> io::Result<()> {
                Buddy::default().decode_encode_decode(super::$data, ops::encode_writer_bytes)
            }
        }
    };
}

test_codec!(flat00_html, CORPUS_HTML, CORPUS_HTML_HASH);
test_codec!(flat01_urls, CORPUS_URLS_10K, CORPUS_URLS_10K_HASH);
test_codec!(flat02_jpg, CORPUS_FIREWORKS, CORPUS_FIREWORKS_HASH);
test_codec!(flat03_jpg_200, CORPUS_FIREWORKS, CORPUS_FIREWORKS_HASH);
test_codec!(flat04_pdf, CORPUS_PAPER_100K, CORPUS_PAPER_100K_HASH);
test_codec!(flat05_html4, CORPUS_HTML_X_4, CORPUS_HTML_X_4_HASH);
test_codec!(flat06_txt1, CORPUS_ALICE29, CORPUS_ALICE29_HASH);
test_codec!(flat07_txt2, CORPUS_ASYOULIK, CORPUS_ASYOULIK_HASH);
test_codec!(flat08_txt3, CORPUS_LCET10, CORPUS_LCET10_HASH);
test_codec!(flat09_txt4, CORPUS_PLRABN12, CORPUS_PLRABN12_HASH);
test_codec!(flat10_pb, CORPUS_GEOPROTO, CORPUS_GEOPROTO_HASH);
test_codec!(flat11_gaviota, CORPUS_KPPKN, CORPUS_KPPKN_HASH);

test_codec!(special_compound, CORPUS_COMPOUND, CORPUS_COMPOUND_HASH);

#[cfg(feature = "large_data")]
test_codec!(large_enwik8, CORPUS_ENWIK8, CORPUS_ENWIK8_HASH);
