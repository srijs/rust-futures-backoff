# futures-backoff

Asynchronous retry strategies based on [futures](https://crates.io/crates/futures).

[![Build Status](https://travis-ci.org/srijs/rust-futures-backoff.svg?branch=master)](https://travis-ci.org/srijs/rust-futures-backoff)
[![crates](http://meritbadge.herokuapp.com/futures-backoff)](https://crates.io/crates/futures-backoff)
[![dependency status](https://deps.rs/repo/github/srijs/rust-futures-backoff/status.svg)](https://deps.rs/repo/github/srijs/rust-futures-backoff)

[Documentation](https://docs.rs/futures-backoff)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
futures-backoff = "0.1"
```

## Examples

```rust
extern crate futures;
extern crate futures_backoff;

use futures::{Future, future};
use futures_backoff::retry;

fn main() {
    let future = retry(|| {
        // do some real-world stuff here...
        future::ok::<u32, ::std::io::Error>(42)
    });

    let result = future.wait();

    assert_eq!(result, Ok(42));
}
```
