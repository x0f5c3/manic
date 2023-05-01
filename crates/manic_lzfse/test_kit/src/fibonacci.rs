/// Fibonacci sequence.
pub struct Fibonacci(u32, u32);

impl Iterator for Fibonacci {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self(0, 0) => None,
            Self(u, 0) => {
                let n = *u;
                *u = 0;
                Some(n)
            }
            Self(u, v) => {
                let n = *u;
                let o = *v;
                *v = n.checked_add(o).unwrap_or(0);
                *u = o;
                Some(n)
            }
        }
    }
}

impl Default for Fibonacci {
    fn default() -> Self {
        Self(0, 1)
    }
}
