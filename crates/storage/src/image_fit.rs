//! The one vocabulary of the image `fit` property. Authored props, resolved props and
//! scenes all carry these names unchanged; the layout engine and the browser renderer
//! are held to the same list by parity tests (`crates/server/tests/image_fit.rs`).
use crate::error::{Result, StorageError};
use serde_json::Value;

/// `fit` stretches the image to the node bounds (CSS `object-fit: fill`), `contain`
/// (the default) scales it to fit inside them and `cover` scales it to fill them, cropping.
pub const IMAGE_FITS: [&str; 3] = ["fit", "contain", "cover"];

/// Reject anything outside [`IMAGE_FITS`], pointing CSS and Figma names at their match.
pub fn check_fit(value: &Value) -> Result<()> {
    let message = match value.as_str() {
        Some(fit) if IMAGE_FITS.contains(&fit) => return Ok(()),
        Some("fill" | "stretch") => {
            "Use \"fit\" to stretch an image to its bounds (CSS object-fit: fill); fit accepts fit, contain or cover"
        }
        Some("none" | "scale-down") => {
            "Use \"contain\" to show the whole image inside its bounds; fit accepts fit, contain or cover"
        }
        _ => "fit accepts fit (stretch to bounds), contain (default, whole image) or cover (fill bounds, crop)",
    };
    Err(StorageError::validation("fit", message))
}
