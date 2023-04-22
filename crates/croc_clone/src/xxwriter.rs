// use std::hash::Hasher;
// use std::io::Write;
// use xxhash_rust::xxh3::Xxh3;
//
// pub struct XXWriter<W: Write> {
//     w: W,
//     hasher: Xxh3,
// }
//
// impl<W: Write> XXWriter<W> {
//     pub fn new(w: W) -> Self {
//         Self {
//             w,
//             hasher: Xxh3::new(),
//         }
//     }
//     pub fn digest(&self) -> u64 {
//         self.hasher.digest()
//     }
//     pub fn into_inner(self) -> W {
//         self.w
//     }
// }
//
// impl<W: Write> Write for XXWriter<W> {
//     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//         self.hasher.write(buf);
//         self.w.write(buf)
//     }
//
//     fn flush(&mut self) -> std::io::Result<()> {
//         self.w.flush()
//     }
// }
