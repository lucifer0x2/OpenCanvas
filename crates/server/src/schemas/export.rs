//! `export_nodes` input: a flat object, like every published tool schema.
use super::{id, node_id, object, text};
use serde_json::{json, Value};

pub const DESCRIPTION: &str = "Export image files of nodes (not a project backup): each node is cut from its root frame laid out at authored size, cropped to its own layout bounds (overflowing children are cut), on a transparent background, drawn alone (its own opacity applies, ancestor clip and opacity do not). settings (1–8, e.g. [{\"scale\":2}]) apply to every node; without them each node uses its exportSettings prop, else one PNG at 1x. A setting is {format?:\"png\", scale?:0.01–4 | width? | height? (pixels, one of the three), suffix?} or {format:\"svg\", suffix?, svgIdAttribute?} (vector, no size keys; svgIdAttribute wraps each node in <g id=nodeId>; text stays live <text>, not outlined, flagged by diagnostics SVG_TEXT_NOT_OUTLINED); the file name is the node name plus suffix, which defaults to \"@<scale>x\" when scale is not 1. Waits up to waitMs (default and max 10000) for the jobs. Each node reports status ready with files (url, fileName, mimeType, scale, width, height, bytes), pending (repeat the identical call; it returns the same jobId) or failed with its own error (NODE_NOT_FOUND, EXPORT_UNSUPPORTED for pages, EMPTY_BOUNDS, PIXEL_BUDGET, EXPORT_SCALE_RANGE, QUEUE_FULL). Download each url over HTTP right away: files live as long as the job record (the last 32 jobs), and no filesystem path is returned.";

/// One export setting; cross-field rules (one size key, distinct file names) are checked by
/// the storage validator shared with the `exportSettings` prop.
fn setting() -> Value {
    let size = json!({"type":"integer","minimum":1,"maximum":16384});
    object(
        json!({"format":{"enum":["png","svg"]},"scale":{"type":"number","minimum":0.01,"maximum":4},"width":size,"height":size,"suffix":{"type":"string","maxLength":64},"svgIdAttribute":{"type":"boolean"}}),
        &[],
    )
}

pub fn schema() -> Value {
    let wait = json!({"type":"integer","minimum":0,"maximum":crate::jobs::MAX_WAIT_MS});
    object(
        json!({"projectId":id(),"nodeIds":{"type":"array","minItems":1,"maxItems":8,"items":node_id()},"settings":{"type":"array","minItems":1,"maxItems":8,"items":setting()},"theme":text(64),"revision":{"type":"integer","minimum":0},"waitMs":wait}),
        &["projectId", "nodeIds"],
    )
}
