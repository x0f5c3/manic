// Fixed constant. Max copy width. Do not change.
pub const WIDE: usize = 32;

// Default copy width. Power of two and <= WIDE.
pub const COPY_WIDTH: usize = 16;

/// # Safety
///
/// * `WIDTH.is_power_of_two()`
/// * `WIDTH <= WIDE`
pub trait Width {
    const WIDTH: usize;
}

#[derive(Copy, Clone, Debug)]
pub struct W00;

impl Width for W00 {
    const WIDTH: usize = COPY_WIDTH;
}

#[derive(Copy, Clone, Debug)]
pub struct W08;

impl Width for W08 {
    const WIDTH: usize = 8;
}

#[derive(Copy, Clone, Debug)]
pub struct W16;

impl Width for W16 {
    const WIDTH: usize = 16;
}

#[derive(Copy, Clone, Debug)]
pub struct Wide;

impl Width for Wide {
    const WIDTH: usize = WIDE;
}
