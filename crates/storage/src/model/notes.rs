//! Notes: design intent attached to a node, stored in the revisioned document.
use super::*;

pub const MAX_NOTES: usize = 5000;
pub const MAX_NOTE_TEXT: usize = 2000;
pub const MAX_NOTE_REPLIES: usize = 100;
pub const MAX_NOTE_AUTHOR: usize = 64;
/// Batch operations that only touch notes; such a commit changes nothing a render shows.
pub const NOTE_OPERATIONS: [&str; 4] = ["create_note", "reply_note", "update_note", "delete_note"];

/// Stored and read as the same JSON; defaults (`resolved:false`, no replies) are omitted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Note {
    pub id: String,
    pub node_id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub resolved: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replies: Vec<NoteReply>,
}

/// One reply, appended to the end of its note's single-level thread.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoteReply {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

/// True when `operations` is a non-empty array whose every `op` is a note operation.
pub fn is_note_batch(operations: &Value) -> bool {
    operations.as_array().is_some_and(|ops| {
        !ops.is_empty()
            && ops.iter().all(|op| {
                op["op"]
                    .as_str()
                    .is_some_and(|name| NOTE_OPERATIONS.contains(&name))
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn note_batches_hold_only_note_operations() {
        for op in NOTE_OPERATIONS {
            assert!(is_note_batch(&json!([{ "op": op }])));
        }
        assert!(is_note_batch(&json!(
            NOTE_OPERATIONS.map(|op| json!({ "op": op }))
        )));
        for other in [
            "update_node",
            "delete_subtree",
            "upsert_token",
            "import_asset",
        ] {
            assert!(!is_note_batch(
                &json!([{"op":"create_note"},{ "op": other }])
            ));
        }
        assert!(!is_note_batch(&json!([])));
        assert!(!is_note_batch(&Value::Null));
    }
}
