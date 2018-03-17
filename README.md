# futures-retry

Extensible, asynchronous retry behaviours based on [futures](https://crates.io/crates/futures).

[![Build Status](https://travis-ci.org/srijs/rust-futures-retry.svg?branch=master)](https://travis-ci.org/srijs/rust-futures-retry)
[![crates](http://meritbadge.herokuapp.com/futures-retry)](https://crates.io/crates/futures-retry)
[![dependency status](https://deps.rs/repo/github/srijs/rust-futures-retry/status.svg)](https://deps.rs/repo/github/srijs/rust-futures-retry)

[Documentation](https://docs.rs/futures-retry)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
futures-retry = "0.1"
```

## Examples

```rust
extern crate futures;
extern crate futures_retry;

use futures::Future;
use futures_retry::Retry;
use futures_retry::strategy::{ExponentialBackoff, jitter};

fn action() -> Result<u64, ()> {
    // do some real-world stuff here...
    Ok(42)
}

fn main() {
    let retry_strategy = ExponentialBackoff::from_millis(10)
        .map(jitter)
        .take(3);

    let retry_future = Retry::spawn(retry_strategy, action);
    let retry_result = retry_future.wait();

    assert_eq!(retry_result, Ok(42));
}
```
