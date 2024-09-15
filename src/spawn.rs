use std::time::Duration;

#[inline]
pub async fn sleep(duration: Duration) {
    timeout_future(duration).await;
}

#[cfg(not(target_os = "unknown"))]
pub use os::*;
#[cfg(not(target_os = "unknown"))]
mod os {
    use std::{
        pin::Pin,
        task::{Context, Poll},
        time::Duration,
    };

    use async_global_executor::Task;
    use futures_lite::{Future, FutureExt};

    pub struct JoinHandle<T> {
        task: Option<Task<T>>,
    }

    impl<T> Drop for JoinHandle<T> {
        fn drop(&mut self) {
            if let Some(task) = self.task.take() {
                task.detach();
            }
        }
    }

    impl<T> Future for JoinHandle<T> {
        type Output = T;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match self.task.as_mut() {
                Some(task) => task.poll(cx),
                None => unreachable!("JoinHandle polled after dropping"),
            }
        }
    }

    pub fn spawn<F>(f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send,
    {
        JoinHandle {
            task: Some(async_global_executor::spawn(f)),
        }
    }

    pub fn spawn_local<F>(f: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
    {
        JoinHandle {
            task: Some(async_global_executor::spawn_local(f)),
        }
    }

    pub fn spawn_blocking<F, T>(f: F) -> impl Future<Output = T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        async_global_executor::spawn_blocking(f)
    }

    pub fn block_on<T>(future: impl Future<Output = T>) -> T {
        async_io::block_on(future)
    }

    pub fn timeout_future(duration: Duration) -> impl Future {
        async_io::Timer::after(duration)
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::time::Duration;

    use futures_lite::Future;

    pub fn spawn<F>(f: F)
    where
        F: Future + 'static,
    {
        spawn_local(f)
    }

    pub fn spawn_local<F>(f: F)
    where
        F: Future + 'static,
    {
        wasm_bindgen_futures::spawn_local(async move {
            f.await;
        });
    }

    pub fn timeout_future(duration: Duration) -> impl Future {
        gloo_timers::future::TimeoutFuture::new(duration.as_millis() as u32)
    }
}
