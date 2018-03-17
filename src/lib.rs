//! This library provides extensible asynchronous retry behaviours
//! for use with the popular [`futures`](https://crates.io/crates/futures) crate.
//!
//! # Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! futures-retry = "0.1"
//! ```
//!
//! # Examples
//!
//! ```rust
//! extern crate futures;
//! extern crate futures_retry;
//!
//! use futures::Future;
//! use futures_retry::Retry;
//! use futures_retry::strategy::{ExponentialBackoff, jitter};
//!
//! fn action() -> Result<u64, ()> {
//!     // do some real-world stuff here...
//!     Ok(42)
//! }
//!
//! fn main() {
//!     let retry_strategy = ExponentialBackoff::from_millis(10)
//!         .map(jitter)
//!         .take(3);
//!
//!     let retry_future = Retry::spawn(retry_strategy, action);
//!     let retry_result = retry_future.wait();
//!
//!     assert_eq!(retry_result, Ok(42));
//! }
//! ```

extern crate futures;
extern crate futures_timer;
extern crate rand;

mod action;
mod condition;
mod future;
/// Assorted retry strategies including fixed interval and exponential back-off.
pub mod strategy;

pub use action::Action;
pub use condition::Condition;
pub use future::{Error, Retry, RetryIf};
