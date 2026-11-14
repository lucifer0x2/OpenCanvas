//! `import_asset` takes the asset bytes from exactly one source: an inbox `path`, inline
//! `base64`, or `sourceProjectId` with `assetId` or `fontId`. As with `render_frame`,
//! clients see the flat union of these shapes while the server validates the strict form,
//! because MCP clients reject a top-level `oneOf`.
use super::{id, object, text};
use serde_json::{json, Map, Value};

pub const DESCRIPTION: &str = "Import one asset (max 10 MiB) from one source: path (a file in the project inbox; preferred, keeps bytes out of the conversation), base64, or sourceProjectId plus assetId or fontId to copy it (fontId keeps its registration; no mediaType). No URLs. Returns asset and commit fields like apply_batch.";
const REQUIRED: [&str; 3] = ["projectId", "expectedRevision", "idempotencyKey"];

fn properties(extra: Value) -> Map<String, Value> {
    let mut properties = json!({"projectId":id(),"expectedRevision":{"type":"integer","minimum":0},"idempotencyKey":id()});
    let properties = properties.as_object_mut().unwrap();
    properties.extend(extra.as_object().unwrap().clone());
    properties.clone()
}
/// Fields of the two byte sources: the declared type and an optional font registration.
fn bytes() -> Value {
    let font = object(
        json!({"id":id(),"family":text(256),"weight":{"type":"integer","minimum":1,"maximum":1000},"style":{"enum":["normal","italic"]},"license":{"type":["string","null"],"maxLength":1024}}),
        &["id", "family", "weight", "style"],
    );
    json!({"mediaType":{"enum":["image/png","image/jpeg","image/webp","font/ttf","font/otf","font/woff2"]},"font":font})
}
fn base64() -> Value {
    json!({"type":"string","minLength":4,"maxLength":13981016})
}
fn shape(extra: Value) -> Value {
    let properties = properties(extra);
    let mut required = REQUIRED.to_vec();
    required.extend(
        properties
            .keys()
            .map(String::as_str)
            .filter(|k| !REQUIRED.contains(k) && *k != "font"),
    );
    object(Value::Object(properties.clone()), &required)
}
fn with_bytes(key: &str, schema: Value) -> Value {
    let mut extra = bytes();
    extra[key] = schema;
    shape(extra)
}
fn copy(key: &str) -> Value {
    shape(json!({"sourceProjectId":text(128),key:text(128)}))
}
/// What `tools/list` shows: every source, none required.
pub fn published() -> Value {
    let mut properties = properties(bytes());
    properties.insert("path".into(), text(4096));
    properties.insert("base64".into(), base64());
    for key in ["sourceProjectId", "assetId", "fontId"] {
        properties.insert(key.into(), text(128));
    }
    object(Value::Object(properties), &REQUIRED)
}
/// What `validate` enforces: exactly one source, each closed to the others.
pub fn validation() -> Value {
    json!({"oneOf":[with_bytes("path", text(4096)), with_bytes("base64", base64()), copy("assetId"), copy("fontId")]})
}

#[cfg(test)]
mod tests;
