use std::future::Future;

// use futures::future::IntoFuture;

/// An action can be run multiple times and produces a future.
pub trait Action: Unpin {
    /// The future that this action produces.
    type Future: Future<Output = Result<Self::Item, Self::Error>>;
    /// The item that the future may resolve with.
    type Item;
    /// The error that the future may resolve with.
    type Error;

    /// Run this action, returning a future.
    fn run(&mut self) -> Self::Future;
}

impl<I, E, T: Future<Output = Result<I, E>>, F: FnMut() -> T> Action for F
where
    F: Unpin,
{
    type Item = I;
    type Error = E;
    type Future = T;

    fn run(&mut self) -> Self::Future {
        self()
    }
}
