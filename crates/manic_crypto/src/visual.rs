//! This

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

const STEADY_TICK: Duration = Duration::from_millis(250);
use crate::Result;

/// This creates a visual spinner.
///
/// The spinner is used for both encrypting and decrypting, provided the feature is enabled.
pub fn create_spinner() -> Result<ProgressBar> {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(STEADY_TICK));
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.cyan}")?);
    Ok(pb)
}
