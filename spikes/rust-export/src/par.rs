//! Data parallelism for the painter and the PNG demultiply, behind the default `parallel` feature
//! (rayon). Without it (wasm32 has no threads) the same helpers run each closure in order, so the
//! output never depends on the thread count; `paint-gate` checks that byte for byte.

/// Bytes per work item for whole-buffer passes: large enough to amortize scheduling, small
/// enough to balance across cores (a 2080×1440 RGBA canvas is ~180 items).
pub const CHUNK: usize = 64 * 1024;

#[cfg(feature = "parallel")]
mod imp {
    use rayon::prelude::*;

    /// A fixed-size worker pool; everything run under `install` uses it.
    pub struct Pool {
        inner: rayon::ThreadPool,
    }

    impl Pool {
        /// `n` workers, 0 = one per core (`RAYON_NUM_THREADS` if set).
        pub fn new(n: usize) -> Pool {
            Pool { inner: rayon::ThreadPoolBuilder::new().num_threads(n).build().expect("thread pool") }
        }
        pub fn threads(&self) -> usize {
            self.inner.current_num_threads()
        }
        pub fn install<R: Send>(&self, f: impl FnOnce() -> R + Send) -> R {
            self.inner.install(f)
        }
    }

    /// Threads of the pool the caller runs in (the global pool outside `install`).
    pub fn threads() -> usize {
        rayon::current_num_threads()
    }

    /// `items.iter().map(f).collect()`, items spread over the pool, results in item order.
    pub fn map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync + Send) -> Vec<R> {
        items.par_iter().map(f).collect()
    }

    /// `f(dst piece, src piece)` over aligned pieces: `dc` items of `dst` with `sc` items of `src`.
    pub fn zip_chunks<A: Send, B: Sync>(dst: &mut [A], dc: usize, src: &[B], sc: usize, f: impl Fn(&mut [A], &[B]) + Sync + Send) {
        assert_eq!(dst.len().div_ceil(dc), src.len().div_ceil(sc), "pieces must pair up");
        dst.par_chunks_mut(dc).zip(src.par_chunks(sc)).for_each(|(d, s)| f(d, s));
    }
}

#[cfg(not(feature = "parallel"))]
mod imp {
    /// Sequential stand-in: one "thread", closures run in order on the caller.
    pub struct Pool;

    impl Pool {
        pub fn new(_n: usize) -> Pool {
            Pool
        }
        pub fn threads(&self) -> usize {
            1
        }
        pub fn install<R: Send>(&self, f: impl FnOnce() -> R + Send) -> R {
            f()
        }
    }

    pub fn threads() -> usize {
        1
    }

    pub fn map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync + Send) -> Vec<R> {
        items.iter().map(f).collect()
    }

    pub fn zip_chunks<A: Send, B: Sync>(dst: &mut [A], dc: usize, src: &[B], sc: usize, f: impl Fn(&mut [A], &[B]) + Sync + Send) {
        assert_eq!(dst.len().div_ceil(dc), src.len().div_ceil(sc), "pieces must pair up");
        dst.chunks_mut(dc).zip(src.chunks(sc)).for_each(|(d, s)| f(d, s));
    }
}

pub use imp::{map, threads, zip_chunks, Pool};

/// Thread count from `--threads N` or `RUST_SPIKE_THREADS` (0 or unset = one per core).
pub fn requested(args: &[String]) -> usize {
    let flag = args.iter().position(|a| a == "--threads").map(|i| args[i + 1].clone());
    flag.or_else(|| std::env::var("RUST_SPIKE_THREADS").ok()).map(|v| v.parse().expect("thread count")).unwrap_or(0)
}
