use crate::lmd::LmdMax;

use super::match_unit::MatchUnit;
pub trait BackendType: MatchUnit + LmdMax {}
