use std::iter::Iterator;
use std::u32::MAX as U32_MAX;

#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    curr: u32,
    base: u32
}

impl ExponentialBackoff {
    pub fn new() -> ExponentialBackoff {
        ExponentialBackoff {
            curr: 1,
            base: 2
        }
    }
}

impl Iterator for ExponentialBackoff {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        let factor = self.curr;

        if let Some(next) = self.curr.checked_mul(self.base) {
            self.curr = next;
        } else {
            self.curr = U32_MAX;
        }

        Some(factor)
    }
}
