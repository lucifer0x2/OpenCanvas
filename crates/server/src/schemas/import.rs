//! `import_asset` takes the asset bytes from exactly one source: an inbox `path` or inline
//! `base64`. As with `render_frame`, clients see the flat union of both shapes while the
//! server validates the strict form, because MCP clients reject a top-level `oneOf`.
use super::{id, object, text};
use serde_json::{json, Map, Value};

pub const DESCRIPTION: &str = "Import one asset (max 10 MiB) from exactly one of path or base64. Prefer path: a file saved in the project's inbox, inbox-relative or absolute inside it (traversal and symlinks forbidden), so its bytes never enter the conversation; base64 is only for clients without filesystem access. Bytes must match mediaType's signature. Replay and the asset hash use the bytes, not the source. Returns asset plus, as apply_batch does, committedRevision, changedIds, renderStatus and replayed at the top level (result repeats them).";
const REQUIRED: [&str; 4] = [
    "projectId",
    "expectedRevision",
    "idempotencyKey",
    "mediaType",
];

fn common() -> Map<String, Value> {
    let font = object(
        json!({"id":id(),"family":text(256),"weight":{"type":"integer","minimum":1,"maximum":1000},"style":{"enum":["normal","italic"]},"license":{"type":["string","null"],"maxLength":1024}}),
        &["id", "family", "weight", "style"],
    );
    let properties = json!({"projectId":id(),"expectedRevision":{"type":"integer","minimum":0},"idempotencyKey":id(),"mediaType":{"enum":["image/png","image/jpeg","image/webp","font/ttf","font/otf","font/woff2"]},"font":font});
    properties.as_object().unwrap().clone()
}
fn source(key: &str, schema: Value) -> Value {
    let mut properties = common();
    properties.insert(key.into(), schema);
    let mut required = REQUIRED.to_vec();
    required.push(key);
    object(Value::Object(properties), &required)
}
fn base64() -> Value {
    json!({"type":"string","minLength":4,"maxLength":13981016})
}
/// What `tools/list` shows: both sources, neither required.
pub fn published() -> Value {
    let mut properties = common();
    properties.insert("path".into(), text(4096));
    properties.insert("base64".into(), base64());
    object(Value::Object(properties), &REQUIRED)
}
/// What `validate` enforces: exactly one source, each closed to the other.
pub fn validation() -> Value {
    json!({"oneOf":[source("path", text(4096)), source("base64", base64())]})
}

#[cfg(test)]
mod tests {
    use super::{published, validation};
    use serde_json::{json, Value};

    fn check(args: Value) -> Result<(), String> {
        crate::schemas::validate("import_asset", &args).map_err(|e| e.0.code)
    }
    fn args(source: Value) -> Value {
        let mut args = json!({"projectId":"p","expectedRevision":0,"idempotencyKey":"k","mediaType":"image/png"});
        args.as_object_mut()
            .unwrap()
            .extend(source.as_object().unwrap().clone());
        args
    }
    #[test]
    fn published_schema_is_the_flat_union_of_both_sources() {
        let published = published();
        let mut union = serde_json::Map::new();
        for shape in validation()["oneOf"].as_array().unwrap() {
            for (key, field) in shape["properties"].as_object().unwrap() {
                assert!(union
                    .insert(key.clone(), field.clone())
                    .is_none_or(|f| f == *field));
            }
        }
        assert_eq!(published["properties"], Value::Object(union));
        assert_eq!(
            published["required"],
            json!([
                "projectId",
                "expectedRevision",
                "idempotencyKey",
                "mediaType"
            ])
        );
        assert_eq!(published["additionalProperties"], false);
    }
    #[test]
    fn validation_requires_exactly_one_source() {
        let font = json!({"id":"f","family":"F","weight":400,"style":"normal"});
        for accepted in [
            args(json!({"path":"logo.png"})),
            args(json!({"path":"nested/logo.png"})),
            args(json!({"base64":"iVBORw=="})),
            args(json!({"path":"f.woff2","font":font})),
        ] {
            assert_eq!(check(accepted.clone()), Ok(()), "{accepted}");
        }
        for rejected in [
            args(json!({})),
            args(json!({"path":"logo.png","base64":"iVBORw=="})),
            args(json!({"path":""})),
            args(json!({"path":7})),
            args(json!({"path":null})),
            args(json!({"base64":["iVBORw=="]})),
            args(json!({"base64":"abc"})),
            args(json!({"path":"logo.png","extra":1})),
            json!({"projectId":"p","expectedRevision":0,"idempotencyKey":"k","path":"logo.png"}),
            args(json!({"path":"logo.png","mediaType":"image/gif"})),
        ] {
            assert_eq!(
                check(rejected.clone()),
                Err("INVALID_REQUEST".into()),
                "{rejected}"
            );
        }
    }
}
