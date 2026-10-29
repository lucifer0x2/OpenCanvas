//! `render_frame` takes one of two exclusive argument shapes: enqueue a render, or poll a job.
//! MCP requires a tool input schema to be a plain `"type":"object"`, and the Anthropic API
//! rejects top-level `oneOf`/`anyOf`/`allOf`, so clients are shown the flat union of both
//! shapes while the server keeps validating the strict two-mode form.
use super::{id, object, text, viewport};
use serde_json::{json, Map, Value};

pub const DESCRIPTION: &str = "Enqueue immutable exact-revision rendering; returns bounded job and HTTP PNG resource metadata when ready. Call it in one of two modes: enqueue with projectId, revision, frameId and viewport (theme, state and scale optional), or poll with projectId and jobId (waitMs optional); arguments mixing the two modes are rejected. Poll using jobId; pass waitMs (0–10000, default 0) to have the poll return as soon as the job leaves pending, or its current state when the wait expires, instead of polling in a tight loop. waitMs is only accepted with jobId. The PNG is the exact pixels the design has at that revision and viewport, so it is also how an implementation is compared against the design. The response leaves out the computed scene (every node, line, font and asset; tens to hundreds of KB) and keeps status, sceneKey, imageUrl/image, provenance, diagnostics and readiness: read bounds and text lines with inspect_layout for that sceneKey, or pass includeScene:true (either mode) for the full scene inline.";
const ENQUEUE_REQUIRED: [&str; 4] = ["projectId", "revision", "frameId", "viewport"];
const POLL_REQUIRED: [&str; 2] = ["projectId", "jobId"];

/// Shared by both modes: the response omits the computed scene unless this is true.
fn include_scene() -> Value {
    json!({"type":"boolean"})
}
fn enqueue() -> Map<String, Value> {
    let properties = json!({"projectId":id(),"revision":{"type":"integer","minimum":0},"frameId":id(),"viewport":viewport(),"theme":text(64),"state":{"enum":["default"]},"scale":{"type":"number","exclusiveMinimum":0,"maximum":4},"includeScene":include_scene()});
    properties.as_object().unwrap().clone()
}
fn poll() -> Map<String, Value> {
    let wait = json!({"type":"integer","minimum":0,"maximum":crate::jobs::MAX_WAIT_MS});
    let properties =
        json!({"projectId":id(),"jobId":id(),"waitMs":wait,"includeScene":include_scene()});
    properties.as_object().unwrap().clone()
}
/// What `tools/list` shows: every field of both modes, only `projectId` required.
pub fn published() -> Value {
    let mut properties = enqueue();
    properties.extend(poll());
    object(Value::Object(properties), &["projectId"])
}
/// What `validate` enforces: exactly one of the two modes, each closed to the other's fields.
pub fn validation() -> Value {
    let enqueue = object(Value::Object(enqueue()), &ENQUEUE_REQUIRED);
    let poll = object(Value::Object(poll()), &POLL_REQUIRED);
    json!({"oneOf":[enqueue, poll]})
}

#[cfg(test)]
mod tests {
    use super::{published, validation};
    use serde_json::{json, Value};

    fn check(args: Value) -> Result<(), String> {
        crate::schemas::validate("render_frame", &args).map_err(|e| e.0.code)
    }
    #[test]
    fn published_schema_is_the_flat_union_of_both_modes() {
        let published = published();
        let mut union = serde_json::Map::new();
        for mode in validation()["oneOf"].as_array().unwrap() {
            for (key, field) in mode["properties"].as_object().unwrap() {
                // A field shared by both modes must carry the same constraints in each.
                assert!(union
                    .insert(key.clone(), field.clone())
                    .is_none_or(|f| f == *field));
            }
        }
        assert_eq!(published["properties"], Value::Object(union));
        assert_eq!(published["type"], "object");
        assert_eq!(published["required"], json!(["projectId"]));
        assert_eq!(published["additionalProperties"], false);
    }
    #[test]
    fn validation_keeps_the_two_exclusive_modes() {
        let viewport = json!({"width":320,"height":200});
        let enqueue = json!({"projectId":"p","revision":1,"frameId":"f","viewport":viewport});
        for accepted in [
            enqueue.clone(),
            json!({"projectId":"p","revision":1,"frameId":"f","viewport":viewport,"theme":"night","state":"default","scale":2}),
            json!({"projectId":"p","jobId":"j"}),
            json!({"projectId":"p","jobId":"j","waitMs":0}),
            json!({"projectId":"p","jobId":"j","waitMs":10_000}),
            json!({"projectId":"p","jobId":"j","includeScene":true}),
            json!({"projectId":"p","revision":1,"frameId":"f","viewport":viewport,"includeScene":false}),
        ] {
            assert_eq!(check(accepted.clone()), Ok(()), "{accepted}");
        }
        let mut rejected = vec![
            json!({"projectId":"p","jobId":"j","frameId":"f"}),
            json!({"projectId":"p","jobId":"j","revision":1}),
            json!({"projectId":"p","jobId":"j","viewport":viewport}),
            json!({"projectId":"p","waitMs":100}),
            json!({"projectId":"p"}),
            json!({}),
            json!({"projectId":"p","jobId":"j","extra":1}),
            json!({"projectId":"p","jobId":"j","waitMs":10_001}),
            json!({"projectId":"p","jobId":"j","waitMs":1.5}),
            json!({"projectId":"p","jobId":"j","includeScene":"yes"}),
            json!({"projectId":"p","includeScene":true}),
        ];
        for extra in [
            json!({"jobId":"j"}),
            json!({"waitMs":1}),
            json!({"extra":1}),
        ] {
            let mut args = enqueue.clone();
            args.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            rejected.push(args);
        }
        for args in rejected {
            assert_eq!(check(args.clone()), Err("INVALID_REQUEST".into()), "{args}");
        }
    }
}
