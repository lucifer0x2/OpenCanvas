#![allow(dead_code)]
//! Note operation builders shared by the notes tests.
use opencanvas_storage::*;
use serde_json::json;

pub fn note(id: &str, node: &str, text: &str) -> Operation {
    Operation::CreateNote {
        id: id.into(),
        node_id: node.into(),
        text: text.into(),
        author: Some("designer".into()),
    }
}
pub fn reply(id: &str, text: &str) -> Operation {
    Operation::ReplyNote {
        id: id.into(),
        text: text.into(),
        author: Some("coder".into()),
    }
}
pub fn update(id: &str, resolved: Option<bool>) -> Operation {
    Operation::UpdateNote {
        id: id.into(),
        text: None,
        resolved,
        node_id: None,
    }
}
pub fn notes_json(p: &Project) -> serde_json::Value {
    json!(p.snapshot().unwrap().notes)
}
