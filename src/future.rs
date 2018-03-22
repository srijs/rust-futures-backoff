use std::io::Error;
use std::fmt;
use std::time::Instant;

use futures::{Async, Future, Poll};
use futures_timer::{Delay, TimerHandle};

use super::strategy::{Strategy, StrategyIter};
use super::action::Action;
use super::condition::Condition;

enum RetryState<A> where A: Action {
    Running(A::Future),
    Sleeping(Delay)
}

impl<A: Action> RetryState<A> {
    fn poll(&mut self) -> RetryFuturePoll<A> {
        match *self {
            RetryState::Running(ref mut future) =>
                RetryFuturePoll::Running(future.poll()),
            RetryState::Sleeping(ref mut future) =>
                RetryFuturePoll::Sleeping(future.poll())
        }
    }
}

enum RetryFuturePoll<A> where A: Action {
    Running(Poll<A::Item, A::Error>),
    Sleeping(Poll<(), Error>)
}

/// Future that drives multiple attempts at an action via a retry strategy.
pub struct Retry<A> where A: Action {
    retry_if: RetryIf<A, fn(&A::Error) -> bool>
}

impl<A: Action> Retry<A> {
    /// Creates a new retry future.
    pub fn new(strategy: &Strategy, action: A) -> Retry<A> {
        Retry::new_with_handle(TimerHandle::default(), strategy, action)
    }

    /// Creates a new retry future, using the provided `handle` to schedule timeouts.
    pub fn new_with_handle(handle: TimerHandle, strategy: &Strategy, action: A) -> Retry<A> {
        Retry {
            retry_if: RetryIf::new_with_handle(handle, strategy, action, (|_| true) as fn(&A::Error) -> bool)
        }
    }
}

impl<A: Action> fmt::Debug for Retry<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Retry").finish()
    }
}

impl<A: Action> Future for Retry<A> {
    type Item = A::Item;
    type Error = A::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.retry_if.poll()
    }
}

/// Future that drives multiple attempts at an action via a retry strategy. Retries are only attempted if
/// the `Error` returned by the future satisfies a given condition.
pub struct RetryIf<A, C>
    where A: Action,
          C: Condition<A::Error>
{
    strategy_iter: StrategyIter,
    state: RetryState<A>,
    action: A,
    handle: TimerHandle,
    condition: C
}

impl<A, C> RetryIf<A, C>
    where A: Action,
          C: Condition<A::Error>
{
    /// Creates a new retry future.
    pub fn new(
        strategy: &Strategy,
        action: A,
        condition: C
    ) -> RetryIf<A, C> {
        RetryIf::new_with_handle(TimerHandle::default(), strategy, action, condition)
    }

    /// Creates a new retry future, using the provided `handle` to schedule timeouts.
    pub fn new_with_handle(
        handle: TimerHandle,
        strategy: &Strategy,
        mut action: A,
        condition: C
    ) -> RetryIf<A, C> {
        RetryIf {
            strategy_iter: strategy.iter(),
            state: RetryState::Running(action.run()),
            action: action,
            handle: handle,
            condition: condition,
        }
    }

    fn attempt(&mut self) -> Poll<A::Item, A::Error> {
        let future = self.action.run();
        self.state = RetryState::Running(future);
        self.poll()
    }

    fn retry(&mut self, err: A::Error) -> Poll<A::Item, A::Error> {
        match self.strategy_iter.next() {
            None => Err(err),
            Some(duration) => {
                let instant = Instant::now() + duration;
                let future = Delay::new_handle(instant, self.handle.clone());
                self.state = RetryState::Sleeping(future);
                self.poll()
            }
        }
    }
}

impl<A: Action, C: Condition<A::Error>> fmt::Debug for RetryIf<A, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RetryIf").finish()
    }
}

impl<A, C> Future for RetryIf<A, C>
    where A: Action,
          C: Condition<A::Error>
{
    type Item = A::Item;
    type Error = A::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.state.poll() {
            RetryFuturePoll::Running(poll_result) => match poll_result {
                Ok(async) => Ok(async),
                Err(err) => {
                    if self.condition.should_retry(&err) {
                        self.retry(err)
                    } else {
                        Err(err)
                    }
                }
            },
            RetryFuturePoll::Sleeping(poll_result) => match poll_result.unwrap() {
                Async::NotReady => Ok(Async::NotReady),
                Async::Ready(_) => self.attempt()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use futures::Future;
    use super::Strategy;

    #[test]
    fn attempts_just_once() {
        let s = Strategy::fixed(Duration::from_millis(100))
            .with_max_retries(0);
        let mut num_calls = 0;
        let res = {
            let fut = s.retry(|| {
                num_calls += 1;
                Err::<(), u64>(42)
            });
            fut.wait()
        };

        assert_eq!(res, Err(42));
        assert_eq!(num_calls, 1);
    }

    #[test]
    fn attempts_until_max_retries_exceeded() {
        let s = Strategy::fixed(Duration::from_millis(100))
            .with_max_retries(2);
        let mut num_calls = 0;
        let res = {
            let fut = s.retry(|| {
                num_calls += 1;
                Err::<(), u64>(42)
            });
            fut.wait()
        };

        assert_eq!(res, Err(42));
        assert_eq!(num_calls, 3);
    }

    #[test]
    fn attempts_until_success() {
        let s = Strategy::fixed(Duration::from_millis(100));
        let mut num_calls = 0;
        let res = {
            let fut = s.retry(|| {
                num_calls += 1;
                if num_calls < 4 {
                    Err::<(), u64>(42)
                } else {
                    Ok::<(), u64>(())
                }
            });
            fut.wait()
        };

        assert_eq!(res, Ok(()));
        assert_eq!(num_calls, 4);
    }

    #[test]
    fn attempts_retry_only_if_given_condition_is_true() {
        let s = Strategy::fixed(Duration::from_millis(100))
            .with_max_retries(5);
        let mut num_calls = 0;
        let res = {
            let action = || {
                num_calls += 1;
                Err::<(), u64>(num_calls)
            };
            let fut = s.retry_if(action, |e: &u64| *e < 3);
            fut.wait()
        };

        assert_eq!(res, Err(3));
        assert_eq!(num_calls, 3);
    }
}
