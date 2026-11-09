use super::*;

pub(super) const INLINE_DESCRIPTION: &str = "Atomically apply 1–500 ordered JSON operations (2 MiB). For edits inside existing pages, supply pageIds plus expectedRevision from the actual authored read: storage guards those pages and transitive shared dependencies while allowing unrelated commits. Omitting pageIds keeps strict project revision matching. Scoped mode rejects shared-resource/note/page/global-flow writes and root deletion/reparent/reorder; use strict mode after rereading. Replay identical key/body after lost responses; changed bodies need new keys. REVISION_CONFLICT requires reread/rebase. dryRun:true validates without writes; scoped success returns editGuard.";
pub(super) const FILE_DESCRIPTION: &str = "Apply one pure JSON operation array from the project's inbox, with apply_batch validation, revision and idempotency. Optional pageIds enables existing-page concurrency using expectedRevision from the actual authored read; storage derives dependencies. path is inbox-relative or absolute inside that inbox; traversal and symlinks forbidden. Max 2 MiB, 1–500 operations; no code execution or automatic chunking. Hash uses canonical content, not filename. Replay exact key/body after lost responses; changed bodies need new keys. dryRun:true previews without writes.";

fn common() -> Value {
    json!({"projectId":id(),"expectedRevision":{"type":"integer","minimum":0},
        "pageIds":{"type":"array","minItems":1,"maxItems":100,"uniqueItems":true,"items":id()},
        "idempotencyKey":id(),"dryRun":{"type":"boolean"}})
}
pub(super) fn inline() -> Value {
    let mut properties = common();
    properties["operations"] =
        json!({"type":"array","minItems":1,"maxItems":500,"items":operations::schema()});
    object(
        properties,
        &[
            "projectId",
            "expectedRevision",
            "idempotencyKey",
            "operations",
        ],
    )
}
pub(super) fn file() -> Value {
    let mut properties = common();
    properties["path"] = text(4096);
    object(
        properties,
        &["projectId", "expectedRevision", "idempotencyKey", "path"],
    )
}
