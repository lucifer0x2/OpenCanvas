//! The `exportSettings` node property: which image files a node is exported as. The same rules
//! check the authored prop and the `settings` argument of the server's `export_nodes` tool, and
//! `export_suffix` names the files, so a setting that validates never collides on file name.
use crate::error::{Result, StorageError};
use serde_json::{Map, Value};

/// Most settings on one node or in one export call.
pub const MAX_EXPORT_SETTINGS: usize = 8;
/// Formats an export setting may name; the first is the default.
pub const EXPORT_FORMATS: &[&str] = &["png"];
const KEYS: [&str; 5] = ["format", "scale", "width", "height", "suffix"];
const MAX_SUFFIX_BYTES: usize = 64;
const MAX_OUTPUT_PIXELS: u64 = 16_384;

/// Checks a settings array; `field` names it in errors (`exportSettings`, `settings`).
pub fn check_export_settings(value: &Value, field: &str) -> Result<()> {
    let items = value.as_array().ok_or_else(|| {
        StorageError::validation(field, "Export settings must be an array of setting objects")
    })?;
    if items.len() > MAX_EXPORT_SETTINGS {
        return Err(StorageError::validation(
            field,
            "At most 8 export settings are supported",
        ));
    }
    let mut names = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let at = format!("{field}[{index}]");
        let setting = item
            .as_object()
            .ok_or_else(|| StorageError::validation(&at, "Export setting must be an object"))?;
        check_setting(setting, &at)?;
        let name = (export_format(setting), export_suffix(setting));
        if names.contains(&name) {
            return Err(StorageError::validation(
                &at,
                "Two export settings produce the same file name; give one a distinct suffix",
            ));
        }
        names.push(name);
    }
    Ok(())
}

fn check_setting(setting: &Map<String, Value>, at: &str) -> Result<()> {
    for key in setting.keys() {
        if !KEYS.contains(&key.as_str()) {
            let hint = match key.as_str() {
                "constraint" => "; use scale, width or height",
                "type" => "; did you mean 'format'?",
                _ => "",
            };
            let message = format!(
                "Unsupported export setting key '{key}'{hint} (allowed: {})",
                KEYS.join(", ")
            );
            return Err(StorageError::validation(&format!("{at}.{key}"), &message));
        }
    }
    if let Some(format) = setting.get("format") {
        if !format.as_str().is_some_and(|f| EXPORT_FORMATS.contains(&f)) {
            let message = format!("Supported export formats: {}", EXPORT_FORMATS.join(", "));
            return Err(StorageError::validation(&format!("{at}.format"), &message));
        }
    }
    let sizes = ["scale", "width", "height"]
        .into_iter()
        .filter(|key| setting.contains_key(*key))
        .count();
    if sizes > 1 {
        return Err(StorageError::validation(
            at,
            "Use only one of scale, width or height",
        ));
    }
    if let Some(scale) = setting.get("scale") {
        if !scale
            .as_f64()
            .is_some_and(|s| s.is_finite() && (0.01..=4.0).contains(&s))
        {
            return Err(StorageError::validation(
                &format!("{at}.scale"),
                "Scale must be a number from 0.01 to 4",
            ));
        }
    }
    for key in ["width", "height"] {
        if let Some(size) = setting.get(key) {
            if !size
                .as_u64()
                .is_some_and(|s| (1..=MAX_OUTPUT_PIXELS).contains(&s))
            {
                return Err(StorageError::validation(
                    &format!("{at}.{key}"),
                    "Output width or height must be an integer from 1 to 16384 pixels",
                ));
            }
        }
    }
    if let Some(suffix) = setting.get("suffix") {
        let valid = suffix.as_str().is_some_and(|s| {
            s.len() <= MAX_SUFFIX_BYTES
                && !s
                    .chars()
                    .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
        });
        if !valid {
            return Err(StorageError::validation(
                &format!("{at}.suffix"),
                "Suffix must be at most 64 bytes without / \\ : * ? \" < > | or control characters",
            ));
        }
    }
    Ok(())
}

/// The setting's format, `png` when omitted.
pub fn export_format(setting: &Map<String, Value>) -> &str {
    setting
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or(EXPORT_FORMATS[0])
}

/// The file-name suffix: the authored one, else `@<scale>x` for a scale other than 1, else empty.
pub fn export_suffix(setting: &Map<String, Value>) -> String {
    if let Some(suffix) = setting.get("suffix").and_then(Value::as_str) {
        return suffix.into();
    }
    match setting.get("scale").and_then(Value::as_f64) {
        Some(scale) if scale != 1.0 => format!("@{scale}x"),
        _ => String::new(),
    }
}
