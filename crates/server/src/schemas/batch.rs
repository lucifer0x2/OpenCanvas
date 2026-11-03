use super::*;

pub(super) const INLINE_DESCRIPTION: &str = "Atomically apply 1–500 ordered JSON operations (2 MiB); *_note ops keep design intent. Replay identical key/body after lost responses; changed bodies need new keys. REVISION_CONFLICT carries changedSince for rebase. dryRun:true validates the full candidate and returns baseRevision, would-be changedIds and diagnostics without writes, key consumption, events or rendering.";
pub(super) const FILE_DESCRIPTION: &str = "Apply one pure JSON operation array from the project's inbox directory, using the same atomic validation/revision/idempotency as apply_batch. path is inbox-relative or an absolute path inside that inbox; traversal and symlinks are forbidden. Max 2 MiB, 1–500 operations; no code execution or automatic chunking. Hash uses canonical content, not filename: changed file bodies require new keys. dryRun:true previews without side effects.";

fn common() -> Value {
    json!({"projectId":id(),"expectedRevision":{"type":"integer","minimum":0},
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
