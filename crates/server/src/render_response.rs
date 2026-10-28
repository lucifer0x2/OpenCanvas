//! What the MCP `render_frame` tool returns. A job record carries the whole computed scene
//! (every node, line, font and asset), tens to hundreds of KB for a real frame, which an
//! agent rarely needs inline: `inspect_layout` reads bounds and lines from the same scene key,
//! paged and selectable. So the tool omits `scene` unless the call passes `includeScene:true`.
//! The job table and the HTTP `/api/v1/jobs/:id` route keep the full record for the viewer.
use crate::{error::Result, state::AppState};
use serde_json::Value;

/// Opt-in field accepted by both `render_frame` modes; removed before the mode is handled.
pub const INCLUDE_SCENE: &str = "includeScene";

/// `render_frame` with its response shaped by [`shape`].
pub(crate) async fn render(state: &AppState, mut args: Value) -> Result<Value> {
    let include = take_include_scene(&mut args);
    crate::mcp::render(state, args)
        .await
        .map(|job| shape(job, include))
}
/// Removes the opt-in flag from the arguments; absent means `false`.
fn take_include_scene(args: &mut Value) -> bool {
    args.as_object_mut()
        .and_then(|object| object.remove(INCLUDE_SCENE))
        .and_then(|flag| flag.as_bool())
        .unwrap_or(false)
}
/// The job record, without `scene` unless it was requested. Every other field (status,
/// sceneKey, imageUrl, image, provenance, diagnostics, readiness, error) is kept as is.
pub fn shape(mut job: Value, include_scene: bool) -> Value {
    if !include_scene {
        if let Some(object) = job.as_object_mut() {
            object.remove("scene");
        }
    }
    job
}

#[cfg(test)]
mod tests {
    use super::{shape, take_include_scene};
    use serde_json::json;

    #[test]
    fn scene_is_dropped_unless_requested() {
        let job = json!({"jobId":"j","status":"ready","sceneKey":"k","scene":{"nodes":[1]},"diagnostics":[]});
        assert_eq!(
            shape(job.clone(), false),
            json!({"jobId":"j","status":"ready","sceneKey":"k","diagnostics":[]})
        );
        assert_eq!(shape(job.clone(), true), job);
        let pending = json!({"jobId":"j","status":"pending","sceneKey":"k"});
        assert_eq!(shape(pending.clone(), false), pending);
    }
    #[test]
    fn flag_is_taken_out_of_the_arguments() {
        let mut args = json!({"projectId":"p","jobId":"j","includeScene":true});
        assert!(take_include_scene(&mut args));
        assert_eq!(args, json!({"projectId":"p","jobId":"j"}));
        let mut args = json!({"projectId":"p","jobId":"j"});
        assert!(!take_include_scene(&mut args));
        let mut args = json!({"projectId":"p","jobId":"j","includeScene":false});
        assert!(!take_include_scene(&mut args));
        assert_eq!(args, json!({"projectId":"p","jobId":"j"}));
    }
}
