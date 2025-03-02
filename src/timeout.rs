use core::fmt;
use std::{
    error::Error,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use pin_project_lite::pin_project;

use crate::timeout_future;

pin_project! {
    pub struct TimeoutFuture<F, D> {
        #[pin]
        future: F,
        #[pin]
        delay: D,
    }
}

impl<F: Future, D: Future> Future for TimeoutFuture<F, D> {
    type Output = Result<F::Output, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.future.poll(cx) {
            Poll::Ready(v) => Poll::Ready(Ok(v)),
            Poll::Pending => match this.delay.poll(cx) {
                Poll::Ready(_) => Poll::Ready(Err(TimeoutError { _private: () })),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

/// An error returned when a future times out.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeoutError {
    _private: (),
}

impl Error for TimeoutError {}

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Future has timed out.".fmt(f)
    }
}

pub trait TimeoutFutureExt
where
    Self: Sized + Future,
{
    fn timeout_when<D>(self, when: D) -> impl Future<Output = Result<Self::Output, TimeoutError>>
    where
        D: Future;

    fn timeout(self, delay: Duration) -> impl Future<Output = Result<Self::Output, TimeoutError>>;
}

impl<F> TimeoutFutureExt for F
where
    F: Future,
{
    fn timeout_when<D>(self, delay: D) -> impl Future<Output = Result<Self::Output, TimeoutError>>
    where
        D: Future,
    {
        TimeoutFuture {
            future: self,
            delay,
        }
    }

    fn timeout(self, delay: Duration) -> impl Future<Output = Result<Self::Output, TimeoutError>> {
        TimeoutFuture {
            future: self,
            delay: timeout_future(delay),
        }
    }
}
