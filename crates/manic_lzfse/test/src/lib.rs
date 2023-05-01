#[cfg(test)]
mod buddy;
#[cfg(test)]
mod data;
#[cfg(test)]
mod fuzz_read;
#[cfg(test)]
mod fuzz_write;
#[cfg(all(test, feature = "huge_data"))]
mod huge;
#[cfg(test)]
mod len;
#[cfg(test)]
mod mutate_0;
#[cfg(test)]
mod mutate_1;
#[cfg(test)]
mod mutate_2;
#[cfg(test)]
mod mutate_3;
#[cfg(test)]
mod mutate_4;
#[cfg(test)]
mod mutate_5;
#[cfg(test)]
mod mutate_6;
#[cfg(test)]
mod mutate_7;
#[cfg(test)]
mod ops;
#[cfg(test)]
mod patchwork_0;
#[cfg(test)]
mod patchwork_1;
#[cfg(test)]
mod pattern_1;
#[cfg(test)]
mod pattern_2;
#[cfg(test)]
mod pattern_3;
#[cfg(test)]
mod pattern_4;
#[cfg(test)]
mod pattern_5;
#[cfg(test)]
mod pattern_6;
#[cfg(test)]
mod random_0;
#[cfg(test)]
mod random_1;
#[cfg(test)]
mod random_2;
