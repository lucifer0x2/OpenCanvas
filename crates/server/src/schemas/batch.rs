//! `apply_batch` takes `operations` (JSON), `source` (OCD, compiled to the same operations), or
//! both (source first, one batch). Like `render_frame`, clients see a flat object because MCP
//! and the Anthropic API reject top-level combinators; the server validates the strict form.
use super::*;

pub(super) const INLINE_DESCRIPTION: &str = "Atomically apply 1–500 ordered JSON operations (2 MiB). update_node merge:true: props are a JSON merge patch (null deletes). Or send source: OCD line DSL (skill://opencanvas/ocd.md) compiled to the same operations, ahead of any operations; parse errors return SOURCE_PARSE_FAILED with line/column. For edits inside existing pages, supply pageIds plus expectedRevision from the actual authored read: storage guards those pages and their shared dependencies, allowing unrelated commits. Omitting pageIds keeps strict project revision matching. Scoped mode rejects shared-resource/note/page/global-flow writes and root deletion/reparent/reorder; use strict mode after rereading. Replay identical key/body after lost responses; changed bodies need new keys. REVISION_CONFLICT requires reread/rebase. dryRun:true validates without writes; scoped success returns editGuard.";
pub(super) const FILE_DESCRIPTION: &str = "Apply one pure JSON operation array from the project's inbox, with apply_batch validation, revision and idempotency. Optional pageIds enables existing-page concurrency using expectedRevision from the actual authored read; storage derives dependencies. path is inbox-relative or absolute inside that inbox; traversal and symlinks forbidden. Max 2 MiB, 1–500 operations; no code execution or automatic chunking. Hash uses canonical content, not filename. Replay exact key/body after lost responses; changed bodies need new keys. dryRun:true previews without writes.";
const REQUIRED: [&str; 3] = ["projectId", "expectedRevision", "idempotencyKey"];
/// OCD source shares the 2 MiB cap of an inline batch. JSON Schema counts characters; the
/// compiler also enforces it in UTF-8 bytes (`ocd::apply`).
const SOURCE_MAX: u64 = 2 * 1024 * 1024;

fn common() -> Value {
    json!({"projectId":id(),"expectedRevision":{"type":"integer","minimum":0},
        "pageIds":{"type":"array","minItems":1,"maxItems":100,"uniqueItems":true,"items":id()},
        "idempotencyKey":id(),"dryRun":{"type":"boolean"}})
}
fn operations() -> Value {
    json!({"type":"array","minItems":1,"maxItems":500,"items":operations::schema()})
}
/// Only called after a failed `apply_batch` validation, to point at the failing operation.
pub(super) fn invalid_operation(items: &[Value]) -> Option<usize> {
    let schema = operations::schema();
    items
        .iter()
        .position(|op| validation::validate(&schema, op).is_err())
}
/// What `tools/list` shows: both inputs, neither required.
pub(super) fn inline() -> Value {
    let mut properties = common();
    properties["operations"] = operations();
    properties["source"] = text(SOURCE_MAX);
    object(properties, &REQUIRED)
}
/// What `validate` enforces: `operations`, `source` or both (the combined count is checked
/// after compiling).
pub(super) fn inline_validation() -> Value {
    let mut with_operations = common();
    with_operations["operations"] = operations();
    let mut with_source = common();
    with_source["source"] = text(SOURCE_MAX);
    let mut with_both = with_operations.clone();
    with_both["source"] = text(SOURCE_MAX);
    let required = |inputs: &[&'static str]| [REQUIRED.as_slice(), inputs].concat();
    json!({"oneOf":[
        object(with_operations, &required(&["operations"])),
        object(with_source, &required(&["source"])),
        object(with_both, &required(&["source", "operations"])),
    ]})
}
pub(super) fn file() -> Value {
    let mut properties = common();
    properties["path"] = text(4096);
    object(
        properties,
        &["projectId", "expectedRevision", "idempotencyKey", "path"],
    )
}
