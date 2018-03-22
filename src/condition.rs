/// Specifies under which conditions a retry is attempted.
pub trait Condition<E> {
    /// Determine whether to retry based on the previous error.
    fn should_retry(&mut self, error: &E) -> bool;
}

impl<E, F: Fn(&E) -> bool> Condition<E> for F {
    fn should_retry(&mut self, error: &E) -> bool {
        self(error)
    }
}
