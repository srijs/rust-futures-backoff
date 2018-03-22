#![deny(missing_docs)]
#![deny(warnings)]
#![deny(missing_debug_implementations)]

//! This library provides asynchronous retry strategies
//! for use with the popular [`futures`](https://crates.io/crates/futures) crate.
//!
//! # Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! futures-backoff = "0.1"
//! ```
//!
//! # Examples
//!
//! ```rust
//! extern crate futures;
//! extern crate futures_backoff;
//!
//! use futures::{Future, future};
//! use futures_backoff::retry;
//!
//! fn main() {
//!     let future = retry(|| {
//!         // do some real-world stuff here...
//!         future::ok::<u32, ::std::io::Error>(42)
//!     });
//!
//!     let result = future.wait();
//!
//!     assert_eq!(result.unwrap(), 42);
//! }
//! ```

extern crate futures;
extern crate futures_timer;
extern crate rand;

mod action;
mod condition;
mod strategy;
mod future;

pub use action::Action;
pub use condition::Condition;
pub use strategy::Strategy;
pub use future::{Retry, RetryIf};

/// Run the given action, and retry on failure.
///
/// Uses the default retry strategy with exponential backoff and a maximum of 5 retry attempts.
///
/// To customize the retry strategy, take a look at [`Strategy`](./struct.Strategy.html).
///
/// # Example
///
/// ```rust
/// # extern crate futures;
/// # extern crate futures_backoff;
/// # use futures::{Future, future};
/// # use futures_backoff::retry;
/// #
/// # fn main() {
/// let future = retry(|| {
///     // do some real-world stuff here...
///     future::ok::<u32, ::std::io::Error>(42)
/// });
/// #
/// # assert_eq!(future.wait().unwrap(), 42);
/// # }
/// ```
pub fn retry<A: Action>(action: A) -> Retry<A> {
    Strategy::default().retry(action)
}

/// Run the given action, and retry on failure if the error satisfies a given condition.
///
/// Uses the default retry strategy with exponential backoff and a maximum of 5 retry attempts.
///
/// To customize the retry strategy, take a look at [`Strategy`](./struct.Strategy.html).
///
/// # Example
///
/// ```rust
/// # extern crate futures;
/// # extern crate futures_backoff;
/// # use std::io::{Error, ErrorKind};
/// # use futures::{Future, future};
/// # use futures_backoff::retry_if;
/// #
/// # fn main() {
/// let future = retry_if(|| {
///     // do some real-world stuff here...
///     future::ok(42)
/// }, |err: &Error| err.kind() == ErrorKind::TimedOut);
/// #
/// # assert_eq!(future.wait().unwrap(), 42);
/// # }
/// ```
pub fn retry_if<A: Action, C>(action: A, condition: C) -> RetryIf<A, C>
    where C: Condition<A::Error>
{
    Strategy::default().retry_if(action, condition)
}
