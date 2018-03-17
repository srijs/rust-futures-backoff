use std::iter::Iterator;

#[derive(Debug, Clone)]
pub struct FixedInterval {}

impl FixedInterval {
    pub fn new() -> FixedInterval {
        FixedInterval {}
    }
}

impl Iterator for FixedInterval {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        Some(1)
    }
}
