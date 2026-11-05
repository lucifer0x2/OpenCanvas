//! Common render limits for native jobs and browser scene preparation.
use crate::{Result, StorageError};

/// Reject unsupported dimensions or a scaled surface above sixteen million pixels.
/// Scene-only callers use scale 1; native render jobs supply their requested scale.
pub fn check_viewport(width: f64, height: f64, scale: f64) -> Result<()> {
    if !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
        || width > 100_000.0
        || height > 100_000.0
        || !scale.is_finite()
        || scale <= 0.0
        || scale > 4.0
        || width * height * scale * scale > 16_000_000.0
    {
        return Err(StorageError::new(
            "INVALID_REQUEST",
            "Viewport or scale exceeds the render budget",
        ));
    }
    Ok(())
}
