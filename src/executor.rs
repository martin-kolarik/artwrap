use std::{
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use async_executor::{Executor, LocalExecutor, Task};
pub(crate) use async_io::block_on;
pub(crate) use blocking::unblock;
use event_listener::Event;

thread_local! {
    static LOCAL_EXECUTOR: LocalExecutor<'static> = LocalExecutor::new();
}

pub(crate) fn spawn_local<F>(f: F) -> Task<F::Output>
where
    F: Future + 'static,
{
    LOCAL_EXECUTOR.with(|executor| executor.spawn(f))
}

static EXECUTOR: OnceLock<Arc<Executor<'static>>> = OnceLock::new();

pub fn executor() -> &'static Executor<'static> {
    EXECUTOR
        .get()
        .expect("Executor not initialized, correct use of the library is to wrap main `with_main`")
}

pub(crate) fn spawn<F>(f: F) -> Task<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    executor().spawn(f)
}

pub fn with_main<T, F: FnOnce() -> T>(f: F) -> T {
    with_main_async(|| async { f() })
}

pub fn with_main_async<T, F: AsyncFnOnce() -> T>(f: F) -> T {
    let _ = EXECUTOR.set(Arc::new(Executor::new()));
    with_thread_pool(|| block_on(executor().run(f())))
}

fn with_thread_pool<T>(f: impl FnOnce() -> T) -> T {
    let stopper = WaitForStop::new();

    thread::scope(|scope| {
        let num_threads = thread::available_parallelism().map_or(1, |num| num.get());
        for i in 0..num_threads {
            let stopper = &stopper;

            thread::Builder::new()
                .name(format!("artwrap-{i}"))
                .spawn_scoped(scope, || {
                    block_on(executor().run(stopper.wait()));
                })
                .expect("failed to spawn thread");
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        stopper.stop();

        match result {
            Ok(value) => value,
            Err(err) => std::panic::resume_unwind(err),
        }
    })
}

struct WaitForStop {
    stopped: AtomicBool,
    events: Event,
}

impl WaitForStop {
    #[inline]
    fn new() -> Self {
        Self {
            stopped: AtomicBool::new(false),
            events: Event::new(),
        }
    }

    #[inline]
    async fn wait(&self) {
        loop {
            if self.stopped.load(Ordering::Relaxed) {
                return;
            }

            event_listener::listener!(&self.events => listener);

            if self.stopped.load(Ordering::Acquire) {
                return;
            }

            listener.await;
        }
    }

    #[inline]
    fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.events.notify_additional(usize::MAX);
    }
}
