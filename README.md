# futures-retry

Asynchronous retry strategies based on [futures](https://crates.io/crates/futures).

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

use futures::{Future, future};
use futures_retry::retry;

fn main() {
    let future = retry(|| {
        // do some real-world stuff here...
        future::ok::<u32, ::std::io::Error>(42)
    });

    let result = future.wait();

    assert_eq!(result, Ok(42));
}
```
