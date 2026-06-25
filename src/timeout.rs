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

#[cfg(not(target_os = "unknown"))]
pub use async_io::Timer as TimeoutFuture;
#[cfg(target_arch = "wasm32")]
pub use gloo_timers::future::TimeoutFuture;

pin_project! {
    pub struct TimeoutableFuture<F> {
        #[pin]
        future: F,
        #[pin]
        delay: TimeoutFuture,
    }
}

impl<F: Future> Future for TimeoutableFuture<F> {
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
    fn timeout(self, delay: Duration) -> TimeoutableFuture<Self>;
}

impl<F> TimeoutFutureExt for F
where
    F: Future,
{
    fn timeout(self, delay: Duration) -> TimeoutableFuture<Self> {
        TimeoutableFuture {
            future: self,
            delay: timeout_future(delay),
        }
    }
}
