pub mod channel {
    pub use async_channel::{Receiver, Sender, bounded, unbounded};
}

#[cfg(not(target_os = "unknown"))]
mod executor;
#[cfg(not(target_os = "unknown"))]
pub use executor::{executor, with_main, with_main_async};

mod spawn;
pub use spawn::*;

mod timeout;
pub use timeout::*;
