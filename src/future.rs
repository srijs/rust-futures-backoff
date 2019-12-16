use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::pin_mut;
use futures_timer::Delay;

use super::action::Action;
use super::condition::Condition;
use super::strategy::{Strategy, StrategyIter};

enum RetryState<A>
where
    A: Action,
{
    Running(A::Future),
    Sleeping(Delay),
}

impl<A> Unpin for RetryState<A> where A: Action {}

impl<A: Action> RetryState<A> {
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> RetryFuturePoll<A> {
        let this = self.get_mut();
        match this {
            RetryState::Running(future) => {
                RetryFuturePoll::Running(unsafe { Pin::new_unchecked(future) }.poll(ctx))
            }
            RetryState::Sleeping(future) => {
                pin_mut!(future);
                RetryFuturePoll::Sleeping(future.poll(ctx))
            }
        }
    }
}

enum RetryFuturePoll<A>
where
    A: Action,
{
    Running(Poll<Result<A::Item, A::Error>>),
    Sleeping(Poll<()>),
}

/// Future that drives multiple attempts at an action via a retry strategy.
pub struct Retry<A>
where
    A: Action,
{
    retry_if: RetryIf<A, fn(&A::Error) -> bool>,
}

impl<A> Unpin for Retry<A> where A: Action {}

impl<A: Action> Retry<A> {
    /// Creates a new retry future.
    pub fn new(strategy: &Strategy, action: A) -> Retry<A> {
        Retry {
            retry_if: RetryIf::new(strategy, action, (|_| true) as fn(&A::Error) -> bool),
        }
    }
}

impl<A: Action> fmt::Debug for Retry<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Retry").finish()
    }
}

impl<A: Action> Future for Retry<A> {
    type Output = Result<A::Item, A::Error>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let retry_if = Pin::new(&mut this.retry_if);
        retry_if.poll(ctx)
    }
}

/// Future that drives multiple attempts at an action via a retry strategy. Retries are only attempted if
/// the `Error` returned by the future satisfies a given condition.
pub struct RetryIf<A, C>
where
    A: Action,
    C: Condition<A::Error>,
{
    strategy_iter: StrategyIter,
    state: RetryState<A>,
    action: A,
    condition: C,
}

impl<A, C> Unpin for RetryIf<A, C>
where
    A: Action,
    C: Condition<A::Error>,
{
}

impl<A, C> RetryIf<A, C>
where
    A: Action,
    C: Condition<A::Error>,
{
    /// Creates a new retry future.
    pub fn new(strategy: &Strategy, mut action: A, condition: C) -> RetryIf<A, C> {
        RetryIf {
            strategy_iter: strategy.iter(),
            state: RetryState::Running(action.run()),
            action: action,
            condition: condition,
        }
    }

    fn attempt(&mut self, ctx: &mut Context<'_>) -> Poll<Result<A::Item, A::Error>> {
        let future = self.action.run();
        self.state = RetryState::Running(future);
        Pin::new(self).poll(ctx)
    }

    fn retry(&mut self, err: A::Error, ctx: &mut Context<'_>) -> Poll<Result<A::Item, A::Error>> {
        match self.strategy_iter.next() {
            None => Poll::Ready(Err(err)),
            Some(duration) => {
                let future = Delay::new(duration);
                self.state = RetryState::Sleeping(future);
                Pin::new(self).poll(ctx)
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
where
    A: Action,
    C: Condition<A::Error>,
{
    type Output = Result<A::Item, A::Error>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match Pin::new(&mut this.state).poll(ctx) {
            RetryFuturePoll::Running(poll_result) => match poll_result {
                Poll::Ready(Ok(a)) => Poll::Ready(Ok(a)),
                Poll::Ready(Err(err)) => {
                    if this.condition.should_retry(&err) {
                        this.retry(err, ctx)
                    } else {
                        Poll::Ready(Err(err))
                    }
                }
                Poll::Pending => Poll::Pending,
            },
            RetryFuturePoll::Sleeping(poll_result) => match poll_result {
                Poll::Pending => Poll::Pending,
                Poll::Ready(_) => this.attempt(ctx),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Strategy;
    use std::time::Duration;

    use futures::executor::block_on;
    use futures::future::Either;

    #[test]
    fn attempts_just_once() {
        let s = Strategy::fixed(Duration::from_millis(100)).with_max_retries(0);
        let mut num_calls = 0;
        let res = {
            let fut = s.retry(|| {
                num_calls += 1;
                async { Err::<(), u64>(42) }
            });
            block_on(fut)
        };

        assert_eq!(res, Err(42));
        assert_eq!(num_calls, 1);
    }

    #[test]
    fn attempts_until_max_retries_exceeded() {
        let s = Strategy::fixed(Duration::from_millis(100)).with_max_retries(2);
        let mut num_calls = 0;
        let res = {
            let fut = s.retry(|| {
                num_calls += 1;
                async { Err::<(), u64>(42) }
            });
            block_on(fut)
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
                    Either::Left(async { Err::<(), u64>(42) })
                } else {
                    Either::Right(async { Ok::<(), u64>(()) })
                }
            });
            block_on(fut)
        };

        assert_eq!(res, Ok(()));
        assert_eq!(num_calls, 4);
    }

    #[test]
    fn attempts_retry_only_if_given_condition_is_true() {
        let s = Strategy::fixed(Duration::from_millis(100)).with_max_retries(5);
        let mut num_calls = 0;
        let res = {
            let action = || {
                num_calls += 1;
                async move { Err::<(), u64>(num_calls) }
            };
            let fut = s.retry_if(action, |e: &u64| *e < 3);
            block_on(fut)
        };

        assert_eq!(res, Err(3));
        assert_eq!(num_calls, 3);
    }
}
