/// # Safety
///

///
/// * `0 < MATCH_UNIT`
/// * `MATCH_UNIT <=4`
pub trait MatchUnit {
    /// Minimum match length, range 1..=4
    const MATCH_UNIT: u32;

    /// Native endian `MATCH_UNIT `bit mask
    const MATCH_MASK: u32;

    fn hash_u(u: u32) -> u32;

    fn match_us(us: (u32, u32)) -> u32;
}
