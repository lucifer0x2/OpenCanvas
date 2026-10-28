use rmcp::model::Tool;
use serde_json::{json, Value};
mod operations;
mod validation;
pub fn validate(name: &str, args: &Value) -> crate::error::Result<()> {
    let tool = tools()
        .into_iter()
        .find(|t| t.name == name)
        .ok_or_else(|| crate::ApiError::new("UNKNOWN_TOOL", "Tool is not available"))?;
    validation::validate(&Value::Object((*tool.input_schema).clone()), args)
}
fn id() -> Value {
    json!({"type":"string","minLength":1,"maxLength":128,"pattern":"^[A-Za-z0-9_-]+$"})
}
fn count(max: u64) -> Value {
    json!({"type":"integer","minimum":1,"maximum":max})
}
fn text(max: u64) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max})
}
pub fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
pub fn tools() -> Vec<Tool> {
    let mut tools = Vec::new();
    let mut add = |name: &'static str, description: String, schema: Value| {
        tools.push(Tool::new(
            name,
            description,
            schema.as_object().unwrap().clone(),
        ));
    };
    add(
        "list_projects",
        "List at most 100 projects, sorted by slug; cursor is the last slug.".into(),
        object(json!({"cursor":text(64),"limit":count(100)}), &[]),
    );
    add(
        "create_project",
        "Create an empty durable project. Does not create demo design data.".into(),
        object(
            json!({"slug":{"type":"string","pattern":"^[a-z0-9]+(-[a-z0-9]+)*$","minLength":1,"maxLength":64},"name":text(256)}),
            &["slug", "name"],
        ),
    );
    let tree = json!({"projectId":id(),"nodeId":id(),"frameId":id(),"depth":{"type":"integer","minimum":0,"maximum":8},"limit":count(200),"budget":{"type":"integer","minimum":1024,"maximum":65536},"properties":{"type":"array","maxItems":50,"items":text(64)},"viewport":viewport(),"theme":text(64),"cursor":text(1024),"revision":{"type":"integer","minimum":0}});
    // Each read says what it is for, so a caller can tell the three scopes apart without trying them.
    const CAPS: &str = " One consistent revision, capped at 200 nodes, depth 8 and 64 KiB; truncation is explicit.";
    for (name, required, purpose) in [
        (
            "get_outline",
            vec!["projectId"],
            "Ordered id/parentId/kind/name/sortKey of the whole project tree. Start here to find the page or frame ID for a scoped read; it carries no props.",
        ),
        (
            "get_subtree",
            vec!["projectId", "nodeId"],
            "Authored props of one node and its descendants. Token and style references stay as authored, unresolved.",
        ),
        (
            "get_design_context",
            vec!["projectId", "frameId"],
            "What implementing one frame needs: its subtree with authored props next to resolvedProps for the requested theme, plus the token and style values behind them. Write code against the authored reference and your own named token for it, so a design theme change stays one change in code; resolvedProps only say what that reference evaluates to today.",
        ),
    ] {
        add(name, format!("{purpose}{CAPS}"), object(tree.clone(), &required));
    }
    let resource = json!({"projectId":id(),"ids":{"type":"array","maxItems":100,"items":id()},"limit":count(100),"cursor":text(1024),"revision":{"type":"integer","minimum":0}});
    for (name, purpose) in [
        ("list_tokens", "Named design tokens and their per-theme values: the system behind the token references in authored props."),
        ("list_styles", "Named styles and the properties each one sets, as referenced by authored props."),
        ("list_assets", "Image and binary asset metadata. Bytes are fetched over the HTTP resource URL; local paths are never returned."),
        ("list_fonts", "Font metadata that text nodes reference by ID, including the bundled font_bundled."),
    ] {
        add(
            name,
            format!("{purpose} One revision, at most 100 items; metadata only, never binary bytes."),
            object(resource.clone(), &["projectId"]),
        );
    }
    add("apply_batch","Atomically apply ordered operations. Replay the identical key/body after lost responses; a changed body requires a new key. Conflicts require readback.".into(),object(json!({"projectId":id(),"expectedRevision":{"type":"integer","minimum":0},"idempotencyKey":id(),"operations":{"type":"array","minItems":1,"maxItems":500,"items":operations::schema()}}),&["projectId","expectedRevision","idempotencyKey","operations"]));
    add(
        "import_asset",
        "Import up to 10 MiB of base64 binary. Paths and arbitrary URLs are not accepted.".into(),
        object(
            json!({"projectId":id(),"expectedRevision":{"type":"integer","minimum":0},"idempotencyKey":id(),"mediaType":{"enum":["image/png","image/jpeg","image/webp","font/ttf","font/otf","font/woff2"]},"base64":{"type":"string","minLength":4,"maxLength":13981016},"font":object(json!({"id":id(),"family":text(256),"weight":{"type":"integer","minimum":1,"maximum":1000},"style":{"enum":["normal","italic"]},"license":{"type":["string","null"],"maxLength":1024}}),&["id","family","weight","style"])}),
            &[
                "projectId",
                "expectedRevision",
                "idempotencyKey",
                "mediaType",
                "base64",
            ],
        ),
    );
    let render = json!({"projectId":id(),"revision":{"type":"integer","minimum":0},"frameId":id(),"viewport":viewport(),"theme":text(64),"state":{"enum":["default"]},"scale":{"type":"number","exclusiveMinimum":0,"maximum":4}});
    add("render_frame","Enqueue immutable exact-revision rendering; returns bounded job and HTTP PNG resource metadata when ready. Poll using jobId. The PNG is the exact pixels the design has at that revision and viewport, so it is also how an implementation is compared against the design.".into(),{
        json!({"oneOf":[object(render,&["projectId","revision","frameId","viewport"]),object(json!({"projectId":id(),"jobId":id()}),&["projectId","jobId"])]})
    });
    add("inspect_layout","Frame-local pixel bounds and the exact text lines of an existing job's scene key: the measured ground truth to check an implementation against, for the viewport that job used. Reading another viewport requires another render. No layout explanation or diff in v1.".into(),object(json!({"projectId":id(),"jobId":id(),"sceneKey":text(128),"nodeIds":{"type":"array","maxItems":200,"items":id()}}),&["projectId","jobId","sceneKey"]));
    tools
}
fn viewport() -> Value {
    object(
        json!({"width":{"type":"number","exclusiveMinimum":0,"maximum":100000},"height":{"type":"number","exclusiveMinimum":0,"maximum":100000}}),
        &["width", "height"],
    )
}
