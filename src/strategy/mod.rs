use std::time::Duration;

use super::{Action, Condition, Retry, RetryIf};

mod exponential_backoff;
mod fibonacci_backoff;
mod fixed_interval;
mod jitter;

pub use self::exponential_backoff::ExponentialBackoff;
pub use self::fibonacci_backoff::FibonacciBackoff;
pub use self::fixed_interval::FixedInterval;
pub use self::jitter::jitter;

#[derive(Debug)]
enum FactorType {
    Exponential,
    Fibonacci,
    Fixed,
}

/// Configurable retry strategy.
///
/// Implements `Default`, which returns an exponential backoff strategy
/// with a delay of 1 second and a maximum of 5 retries.
///
/// # Example
///
/// ```rust
/// # use futures::{Future, future, executor::block_on};
/// # use futures_backoff::Strategy;
/// #
/// # fn main() {
/// let strategy = Strategy::default()
///     .with_max_retries(3);
///
/// let future = strategy.retry(|| {
///     // do some real-world stuff here...
///     async { Ok::<u32, ::std::io::Error>(42) }
/// });
/// #
/// # assert_eq!(block_on(future).unwrap(), 42);
/// # }
/// ```
#[derive(Debug)]
pub struct Strategy {
    factor: FactorType,
    delay: Duration,
    max_delay: Option<Duration>,
    max_retries: usize,
    jitter: bool,
}

impl Default for Strategy {
    fn default() -> Strategy {
        Strategy {
            factor: FactorType::Exponential,
            delay: Duration::from_millis(1000),
            max_delay: None,
            max_retries: 5,
            jitter: false,
        }
    }
}

impl Strategy {
    /// Creates a retry strategy driven by exponential back-off.
    ///
    /// The specified duration will be multiplied by `2^n`, where `n` is
    /// the number of failed attempts.
    pub fn exponential(delay: Duration) -> Strategy {
        Strategy::new(FactorType::Exponential, delay)
    }

    /// Creates a retry strategy driven by a fibonacci back-off.
    ///
    /// The specified duration will be multiplied by `fib(n)`, where `n` is
    /// the number of failed attempts.
    ///
    /// Depending on the problem at hand, a fibonacci retry strategy might
    /// perform better and lead to better throughput than the `ExponentialBackoff`
    /// strategy.
    ///
    /// See ["A Performance Comparison of Different Backoff Algorithms under Different Rebroadcast Probabilities for MANETs."](http://www.comp.leeds.ac.uk/ukpew09/papers/12.pdf)
    /// for more details.
    pub fn fibonacci(delay: Duration) -> Strategy {
        Strategy::new(FactorType::Fibonacci, delay)
    }

    /// Creates a retry strategy driven by a fixed delay.
    pub fn fixed(delay: Duration) -> Strategy {
        Strategy::new(FactorType::Fixed, delay)
    }

    fn new(factor: FactorType, delay: Duration) -> Strategy {
        Strategy {
            factor: factor,
            delay: delay,
            max_delay: None,
            max_retries: 5,
            jitter: false,
        }
    }

    /// Sets the maximum delay between two attempts.
    ///
    /// By default there is no maximum.
    pub fn with_max_delay(mut self, duration: Duration) -> Self {
        self.max_delay = Some(duration);
        self
    }

    /// Sets the maximum number of retry attempts.
    ///
    /// By default a retry will be attempted 5 times before giving up.
    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }

    /// Enables or disables jitter on the delay.
    ///
    /// Jitter will introduce a random variance to the retry strategy,
    /// which can be helpful to mitigate the "Thundering Herd" problem.
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    pub(crate) fn iter(&self) -> StrategyIter {
        let factor_iter = match self.factor {
            FactorType::Exponential => FactorIter::Exponential(ExponentialBackoff::new()),
            FactorType::Fibonacci => FactorIter::Fibonacci(FibonacciBackoff::new()),
            FactorType::Fixed => FactorIter::Fixed(FixedInterval::new()),
        };
        StrategyIter {
            factor_iter: factor_iter,
            delay: self.delay,
            max_delay: self.max_delay,
            retries: self.max_retries,
            jitter: self.jitter,
        }
    }

    /// Run the given action, and use this strategy to retry on failure.
    pub fn retry<A: Action>(&self, action: A) -> Retry<A> {
        Retry::new(self, action)
    }

    /// Run the given action, and use this strategy to retry on failure if the error satisfies a given condition.
    pub fn retry_if<A: Action, C>(&self, action: A, condition: C) -> RetryIf<A, C>
    where
        C: Condition<A::Error>,
    {
        RetryIf::new(self, action, condition)
    }
}

enum FactorIter {
    Exponential(ExponentialBackoff),
    Fibonacci(FibonacciBackoff),
    Fixed(FixedInterval),
}

impl Iterator for FactorIter {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        match self {
            &mut FactorIter::Exponential(ref mut iter) => iter.next(),
            &mut FactorIter::Fibonacci(ref mut iter) => iter.next(),
            &mut FactorIter::Fixed(ref mut iter) => iter.next(),
        }
    }
}

pub(crate) struct StrategyIter {
    factor_iter: FactorIter,
    delay: Duration,
    max_delay: Option<Duration>,
    retries: usize,
    jitter: bool,
}

impl Iterator for StrategyIter {
    type Item = Duration;

    fn next(&mut self) -> Option<Duration> {
        if self.retries > 0 {
            if let Some(factor) = self.factor_iter.next() {
                if let Some(mut delay) = self.delay.checked_mul(factor) {
                    if self.jitter {
                        delay = jitter(delay);
                    }
                    if let Some(max_delay) = self.max_delay {
                        delay = ::std::cmp::min(delay, max_delay);
                    }
                    self.retries -= 1;
                    return Some(delay);
                }
            }
        }
        None
    }
}

#[test]
fn fixed_returns_delay() {
    let mut s = Strategy::fixed(Duration::from_millis(123)).iter();

    assert_eq!(s.next(), Some(Duration::from_millis(123)));
    assert_eq!(s.next(), Some(Duration::from_millis(123)));
    assert_eq!(s.next(), Some(Duration::from_millis(123)));
}

#[test]
fn fibonacci_returns_the_fibonacci_series_starting_at_10() {
    let mut s = Strategy::fibonacci(Duration::from_millis(10)).iter();

    assert_eq!(s.next(), Some(Duration::from_millis(10)));
    assert_eq!(s.next(), Some(Duration::from_millis(10)));
    assert_eq!(s.next(), Some(Duration::from_millis(20)));
    assert_eq!(s.next(), Some(Duration::from_millis(30)));
    assert_eq!(s.next(), Some(Duration::from_millis(50)));
}

#[test]
fn fibonacci_stops_increasing_at_max_delay() {
    let mut s = Strategy::fibonacci(Duration::from_millis(10))
        .with_max_delay(Duration::from_millis(30))
        .iter();

    assert_eq!(s.next(), Some(Duration::from_millis(10)));
    assert_eq!(s.next(), Some(Duration::from_millis(10)));
    assert_eq!(s.next(), Some(Duration::from_millis(20)));
    assert_eq!(s.next(), Some(Duration::from_millis(30)));
    assert_eq!(s.next(), Some(Duration::from_millis(30)));
}

#[test]
fn fibonacci_returns_max_when_max_less_than_base() {
    let mut s = Strategy::fibonacci(Duration::from_millis(10))
        .with_max_delay(Duration::from_millis(10))
        .iter();

    assert_eq!(s.next(), Some(Duration::from_millis(10)));
    assert_eq!(s.next(), Some(Duration::from_millis(10)));
}

#[test]
fn exponential_returns_multiples_of_10ms() {
    let mut s = Strategy::exponential(Duration::from_millis(10)).iter();

    assert_eq!(s.next(), Some(Duration::from_millis(10)));
    assert_eq!(s.next(), Some(Duration::from_millis(20)));
    assert_eq!(s.next(), Some(Duration::from_millis(40)));
}

#[test]
fn exponential_returns_multiples_of_100ms() {
    let mut s = Strategy::exponential(Duration::from_millis(100)).iter();

    assert_eq!(s.next(), Some(Duration::from_millis(100)));
    assert_eq!(s.next(), Some(Duration::from_millis(200)));
    assert_eq!(s.next(), Some(Duration::from_millis(400)));
}

#[test]
fn exponential_stops_increasing_at_max_delay() {
    let mut s = Strategy::exponential(Duration::from_millis(20))
        .with_max_delay(Duration::from_millis(40))
        .iter();

    assert_eq!(s.next(), Some(Duration::from_millis(20)));
    assert_eq!(s.next(), Some(Duration::from_millis(40)));
    assert_eq!(s.next(), Some(Duration::from_millis(40)));
}

#[test]
fn exponential_returns_max_when_max_less_than_base() {
    let mut s = Strategy::exponential(Duration::from_millis(20))
        .with_max_delay(Duration::from_millis(10))
        .iter();

    assert_eq!(s.next(), Some(Duration::from_millis(10)));
    assert_eq!(s.next(), Some(Duration::from_millis(10)));
}
