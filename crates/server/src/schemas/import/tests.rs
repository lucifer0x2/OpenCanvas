use super::{published, validation};
use serde_json::{json, Value};

fn check(args: Value) -> Result<(), String> {
    crate::schemas::validate("import_asset", &args).map_err(|e| e.0.code)
}
fn base(media: bool, source: Value) -> Value {
    let mut args = json!({"projectId":"p","expectedRevision":0,"idempotencyKey":"k"});
    if media {
        args["mediaType"] = json!("image/png");
    }
    args.as_object_mut()
        .unwrap()
        .extend(source.as_object().unwrap().clone());
    args
}
fn args(source: Value) -> Value {
    base(true, source)
}
fn copy(source: Value) -> Value {
    base(false, source)
}

#[test]
fn published_schema_is_the_flat_union_of_every_source() {
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
        json!(["projectId", "expectedRevision", "idempotencyKey"])
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
        copy(json!({"sourceProjectId":"q","assetId":"asset_1"})),
        copy(json!({"sourceProjectId":"q","fontId":"brand"})),
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
        // Copies take the source's type and registration and name exactly one item.
        copy(json!({"sourceProjectId":"q"})),
        copy(json!({"assetId":"asset_1"})),
        copy(json!({"sourceProjectId":"q","assetId":"a","fontId":"f"})),
        args(json!({"sourceProjectId":"q","assetId":"asset_1"})),
        copy(json!({"sourceProjectId":"q","fontId":"f","font":font})),
        copy(json!({"sourceProjectId":"q","assetId":"a","path":"logo.png"})),
        args(json!({"path":"logo.png","assetId":"asset_1"})),
    ] {
        assert_eq!(
            check(rejected.clone()),
            Err("INVALID_REQUEST".into()),
            "{rejected}"
        );
    }
}
