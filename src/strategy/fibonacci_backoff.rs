use std::iter::Iterator;
use std::u32::MAX as U32_MAX;

#[derive(Debug, Clone)]
pub struct FibonacciBackoff {
    curr: u32,
    next: u32
}

impl FibonacciBackoff {
    pub fn new() -> FibonacciBackoff {
        FibonacciBackoff {
            curr: 1,
            next: 1
        }
    }
}

impl Iterator for FibonacciBackoff {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        let factor =  self.curr;

        if let Some(next_next) = self.curr.checked_add(self.next) {
            self.curr = self.next;
            self.next = next_next;
        } else {
            self.curr = self.next;
            self.next = U32_MAX;
        }

        Some(factor)
    }
}
