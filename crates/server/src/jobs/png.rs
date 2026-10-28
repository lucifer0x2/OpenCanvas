use crate::error::{ApiError, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
pub(super) fn decode(value: &Value) -> Result<Vec<u8>> {
    let invalid = || {
        ApiError::new(
            "WORKER_FAILED",
            "Worker returned invalid PNG metadata or bytes",
        )
    };
    let bytes = STANDARD
        .decode(value["base64"].as_str().ok_or_else(invalid)?)
        .map_err(|_| invalid())?;
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return Err(invalid());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if width == 0
        || height == 0
        || u64::from(width) * u64::from(height) > 16_000_000
        || value["width"] != width
        || value["height"] != height
    {
        return Err(invalid());
    }
    Ok(bytes)
}
