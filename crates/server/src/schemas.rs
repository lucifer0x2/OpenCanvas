use rmcp::model::Tool;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::OnceLock};
mod export;
mod operations;
mod render;
mod validation;
/// The catalog is fixed for the life of the process, so tools and their input schemas are built
/// once instead of on every `validate`, `list_tools` and `get_tool`.
fn catalog() -> &'static (Vec<Tool>, HashMap<String, Value>) {
    static CATALOG: OnceLock<(Vec<Tool>, HashMap<String, Value>)> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let tools = build();
        let schemas = tools
            .iter()
            .map(|t| {
                // render_frame publishes a flat object; its arguments are checked in strict form.
                let schema = match t.name.as_ref() {
                    "render_frame" => render::validation(),
                    _ => Value::Object((*t.input_schema).clone()),
                };
                (t.name.to_string(), schema)
            })
            .collect();
        (tools, schemas)
    })
}
pub fn validate(name: &str, args: &Value) -> crate::error::Result<()> {
    let schema = catalog()
        .1
        .get(name)
        .ok_or_else(|| crate::ApiError::new("UNKNOWN_TOOL", "Tool is not available"))?;
    validation::validate(schema, args)
}
pub fn tools() -> Vec<Tool> {
    catalog().0.clone()
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
fn build() -> Vec<Tool> {
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
    const CAPS: &str = " depth (0–8) keeps nodes at most that many levels below the scope root, which is level 0 (project scope: pages are 0, their root frames 1); a node whose children were cut carries omittedChildren. Omitted depth reads the whole scope. One consistent revision, at most 200 nodes and 64 KiB per page; truncation is explicit.";
    for (name, required, purpose) in [
        (
            "get_outline",
            vec!["projectId"],
            "Ordered id/parentId/kind/name/sortKey of the whole project tree. Start here to find the page or frame ID for a scoped read; depth:1 lists every page with its root frames. It carries no props.",
        ),
        (
            "get_subtree",
            vec!["projectId", "nodeId"],
            "Authored props of one node and its descendants. Token and style references stay as authored, unresolved.",
        ),
        (
            "get_design_context",
            vec!["projectId", "frameId"],
            "What implementing one frame needs: its subtree with authored props next to resolvedProps for the requested theme, plus the token and style values behind them. Write code against the authored reference and your own named token for it, so a design theme change stays one change in code; resolvedProps only say what that reference evaluates to today. When the frame has links or flowStart, the first page adds navigation {flowStart, out, in}.",
        ),
    ] {
        let mut properties = tree.clone();
        let mut description = format!("{purpose}{CAPS}");
        if name == "get_design_context" {
            properties["viewports"] = json!({"type":"array","minItems":1,"maxItems":4,"items":viewport()});
            properties["compact"] = json!({"type":"boolean"});
            properties["notes"] = json!({"enum":["open","all","none"]});
            properties["statesMode"] = json!({"enum":["none","resolved"]});
            description.push_str(" statesMode:resolved adds resolvedStates with only visual overrides differing from default; none (default) omits it.");
            description.push_str(" notes lists the intent notes on the frame, its descendants and ancestors: open (default), all (adds resolved:true ones) or none; absent when there are none.");
            description.push_str(" compact:true (default false) shrinks the read: resolvedProps keep only keys whose resolved value differs from authored (resolvedPropsMode:\"differing\"; the full value is authored props without styleId overlaid with them), and tokens/styles/assets/fonts are only those the frame references, closed over style and alias/theme references (resources:\"referenced\"). Diagnostics and reflow still cover the whole frame.");
            description.push_str(" Optional viewports (1–4, mutually exclusive with viewport, total 16M pixels) enqueue cached render jobs without waiting. Repeat the same read when computed.status is pending; ready nodes include computedBounds in viewport order and reflow compares all descendants, excluding the root. Pending/failed comparisons return reflow:null.");
        }
        add(name, description, object(properties, &required));
    }
    let kinds = json!({"enum":["page","frame","group","rectangle","text","image"]});
    add(
        "find_nodes",
        "Find nodes in one call when you know a name, visible text, kind or ID; use get_outline when you need structure. Criteria are ANDed (kinds is OR within the list) and at least one is required. Matching is case-insensitive substring via lowercase, without accent folding or Unicode normalization; textContains only matches text nodes' props.text; parentId matches the direct children of that node, so parentId:<pageId> with kinds:[\"frame\"] lists a page's root frames. Scope is the project, or one nodeId or frameId subtree. Returns preorder matches with id, kind, name, parentId, depth and the ancestor path (up to 8 nearest, pathTruncated when cut), plus textPreview and the requested authored properties. totalMatches and scannedNodes cover the whole scope before paging: totalMatches 0 means nothing matched, not an error; truncated with nextCursor means more pages. Position (left/right) is not searchable: read bounds with get_design_context viewports or inspect_layout nodeIds. One revision, up to 200 matches and 64 KiB per page.".into(),
        object(
            json!({"projectId":id(),"nodeId":id(),"frameId":id(),"kinds":{"type":"array","minItems":1,"maxItems":6,"items":kinds},"nameContains":text(128),"textContains":text(256),"ids":{"type":"array","minItems":1,"maxItems":200,"items":id()},"parentId":id(),"properties":{"type":"array","maxItems":20,"items":text(64)},"limit":count(200),"budget":{"type":"integer","minimum":1024,"maximum":65536},"cursor":text(1024),"revision":{"type":"integer","minimum":0}}),
            &["projectId"],
        ),
    );
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
    add(
        "get_changes",
        "Commits after sinceRevision from the durable log: each revision with its changedIds (operations with includeOperations:true), and their union grouped at currentRevision into nodes/tokens/styles/assets/fonts/notes, or removed when gone. Use it to catch up or to rebase after REVISION_CONFLICT; re-read nodes with find_nodes ids. Up to limit (max 100) whole commits within budget per page; truncated:true means repeat with sinceRevision:nextRevision. Ahead of the project or missing history returns RESET_REQUIRED: re-read the snapshot.".into(),
        object(
            json!({"projectId":id(),"sinceRevision":{"type":"integer","minimum":0},"limit":count(100),"includeOperations":{"type":"boolean"},"budget":{"type":"integer","minimum":1024,"maximum":65536}}),
            &["projectId", "sinceRevision"],
        ),
    );
    add("apply_batch","Atomically apply ordered operations; *_note ops keep design intent on a node (delete_subtree drops its notes). Replay the identical key/body after lost responses; a changed body requires a new key. A REVISION_CONFLICT carries changedSince, the IDs changed after your expectedRevision: rebase on them or read back.".into(),object(json!({"projectId":id(),"expectedRevision":{"type":"integer","minimum":0},"idempotencyKey":id(),"operations":{"type":"array","minItems":1,"maxItems":500,"items":operations::schema()}}),&["projectId","expectedRevision","idempotencyKey","operations"]));
    add(
        "import_asset",
        "Import up to 10 MiB of base64 binary. Paths and arbitrary URLs are not accepted. Returns asset plus, as apply_batch does, committedRevision, changedIds, renderStatus and replayed at the top level (result repeats them).".into(),
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
    add(
        "render_frame",
        render::DESCRIPTION.into(),
        render::published(),
    );
    add("export_nodes", export::DESCRIPTION.into(), export::schema());
    add("inspect_layout","Frame-local pixel bounds and the exact text lines of an existing job's scene key: the measured ground truth to check an implementation against, for the viewport that job used. Reading another viewport requires another render. No layout explanation or diff in v1. Nodes come in scene order, up to limit (default and max 200) per page; budget (bytes) is uncapped unless given. fields picks bounds and/or lines (default both); fields:[\"bounds\"] is much smaller. truncated:true always comes with nextCursor: repeat the same arguments with cursor to read the rest. The cursor binds the job, so later commits do not invalidate it.".into(),object(json!({"projectId":id(),"jobId":id(),"sceneKey":text(128),"nodeIds":{"type":"array","maxItems":200,"items":id()},"fields":{"type":"array","minItems":1,"maxItems":2,"items":{"enum":["bounds","lines"]}},"limit":count(200),"budget":{"type":"integer","minimum":1024,"maximum":65536},"cursor":text(1024)}),&["projectId","jobId","sceneKey"]));
    tools
}
fn viewport() -> Value {
    object(
        json!({"width":{"type":"number","exclusiveMinimum":0,"maximum":100000},"height":{"type":"number","exclusiveMinimum":0,"maximum":100000}}),
        &["width", "height"],
    )
}
