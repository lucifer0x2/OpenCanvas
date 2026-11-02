//! Optional phase timing for the isolated spike; no request data is collected.
use serde::Serialize;
use std::time::Instant;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyProfile {
    pub clone_us: u64,
    pub execute_us: u64,
    pub validate_us: u64,
    pub sort_us: u64,
}

pub(crate) fn measured<T>(output: Option<&mut u64>, action: impl FnOnce() -> T) -> T {
    let started = output.as_ref().map(|_| Instant::now());
    let result = action();
    if let (Some(output), Some(started)) = (output, started) {
        *output = started.elapsed().as_micros().try_into().unwrap_or(u64::MAX);
    }
    result
}
