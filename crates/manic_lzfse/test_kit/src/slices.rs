/// Build incrementing match with the specified arguments.
pub fn build_match_inc(slice: &mut [u8], index: usize, match_index: usize, match_len: usize) {
    assert!(match_index < index);
    assert!(index <= slice.len());
    assert!(match_len <= slice.len() - index);
    let distance = index - match_index;
    assert!(distance <= 255);
    slice.iter_mut().for_each(|u| *u = 0);
    for i in 0..match_len + distance {
        slice[match_index + i] = (i % distance as usize) as u8 + 1;
    }
}

/// Build decrementing match with the specified arguments.
pub fn build_match_dec(slice: &mut [u8], index: usize, match_index: usize, match_len: usize) {
    assert!(match_index < index);
    assert!(index <= slice.len());
    assert!(match_len <= match_index);
    let distance = index - match_index;
    assert!(distance <= 255);
    slice.iter_mut().for_each(|u| *u = 0);
    for i in 0..match_len + distance {
        slice[index - i - 1] = (i % distance as usize) as u8 + 1;
    }
}

pub fn dump_slice(buf: &[u8]) {
    for (i, b) in buf.iter().enumerate() {
        if i % 16 == 0 {
            println!();
            print!("{:04X} - ", i);
        }
        print!("{:02X} ", b);
    }
    println!()
}
