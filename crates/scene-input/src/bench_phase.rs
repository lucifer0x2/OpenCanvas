//! Benchmark-only phase timer for the Task24 large-project harness.
//!
//! Without the `large-project-bench` feature `time` is a plain call and nothing is
//! recorded. With it, each call appends `(phase, microseconds)` to one process-wide
//! list that a sequential benchmark drains with `take` after every measured call.

#[cfg(feature = "large-project-bench")]
static PHASES: std::sync::Mutex<Vec<(&'static str, u64)>> = std::sync::Mutex::new(Vec::new());

/// Runs `f`, recording its duration under `name` only in the benchmark build.
#[inline(always)]
pub fn time<T>(name: &'static str, f: impl FnOnce() -> T) -> T {
    #[cfg(feature = "large-project-bench")]
    {
        let start = std::time::Instant::now();
        let out = f();
        let elapsed = start.elapsed().as_micros() as u64;
        if let Ok(mut phases) = PHASES.lock() {
            phases.push((name, elapsed));
        }
        out
    }
    #[cfg(not(feature = "large-project-bench"))]
    {
        let _ = name;
        f()
    }
}

/// Records the time until it is dropped, for scopes that return early with `?`.
pub struct Span {
    #[cfg(feature = "large-project-bench")]
    name: &'static str,
    #[cfg(feature = "large-project-bench")]
    start: std::time::Instant,
}

impl Span {
    #[inline(always)]
    pub fn new(name: &'static str) -> Self {
        #[cfg(feature = "large-project-bench")]
        {
            Self {
                name,
                start: std::time::Instant::now(),
            }
        }
        #[cfg(not(feature = "large-project-bench"))]
        {
            let _ = name;
            Self {}
        }
    }
}

#[cfg(feature = "large-project-bench")]
impl Drop for Span {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_micros() as u64;
        if let Ok(mut phases) = PHASES.lock() {
            phases.push((self.name, elapsed));
        }
    }
}

/// Drains every phase recorded since the previous call, in recording order.
#[cfg(feature = "large-project-bench")]
pub fn take() -> Vec<(&'static str, u64)> {
    PHASES
        .lock()
        .map(|mut phases| std::mem::take(&mut *phases))
        .unwrap_or_default()
}

#[cfg(all(test, not(feature = "large-project-bench")))]
mod tests {
    #[test]
    fn default_build_keeps_project_caps_and_records_nothing() {
        assert_eq!(crate::MAX_NODES, 20_000);
        assert_eq!(crate::MAX_NOTES, 5_000);
        assert_eq!(super::time("phase", || 7), 7);
        let _span = super::Span::new("phase");
    }
}
