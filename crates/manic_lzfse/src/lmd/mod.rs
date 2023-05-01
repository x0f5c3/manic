mod lmd_non_pack;
mod lmd_pack;
mod lmd_type;

pub use lmd_non_pack::Lmd;
pub use lmd_pack::LmdPack;
pub use lmd_type::*;

#[cfg(test)]
pub fn split_lmd<T: LmdMax>(
    dst: &mut Vec<Lmd<T>>,
    mut literal_len: u32,
    mut match_len: u32,
    match_distance: u32,
) {
    loop {
        if literal_len > T::MAX_LITERAL_LEN as u32 {
            dst.push(Lmd::new(T::MAX_LITERAL_LEN as u32, 0, 1));
            literal_len -= T::MAX_LITERAL_LEN as u32;
        } else if match_len > T::MAX_MATCH_LEN as u32 {
            dst.push(Lmd::new(literal_len, T::MAX_MATCH_LEN as u32, match_distance));
            literal_len = 0;
            match_len -= T::MAX_MATCH_LEN as u32;
        } else {
            dst.push(Lmd::new(literal_len, match_len, match_distance));
            break;
        }
    }
}

#[cfg(test)]
pub fn n_raw_bytes<T: LmdMax>(lmds: &[Lmd<T>]) -> u32 {
    lmds.iter().map(|&Lmd(l, m, _)| l.get() + m.get()).sum()
}
