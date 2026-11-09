//! Clock for the paint profile. wasm32-unknown-unknown has no clock in std (`Instant::now` panics),
//! so there every span reads 0 and the host times the exported calls instead.
#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub struct Instant;

#[cfg(target_arch = "wasm32")]
impl Instant {
    pub fn now() -> Instant {
        Instant
    }
    pub fn elapsed(&self) -> std::time::Duration {
        std::time::Duration::ZERO
    }
}
