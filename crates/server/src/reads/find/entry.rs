use super::{
    super::{index::TreeIndex, ReadRequest},
    criteria::Criteria,
};
use opencanvas_storage::Node;
use serde_json::{json, Value};

const PATH_MAX: usize = 8;
const PREVIEW_CHARS: usize = 120;

pub(super) fn entry(
    index: &TreeIndex,
    root: Option<&str>,
    node: &Node,
    depth: usize,
    matched_on: Vec<&str>,
    criteria: &Criteria,
    req: &ReadRequest,
) -> Value {
    let chain = index.ancestors(&node.id, root);
    let cut = chain.len().saturating_sub(PATH_MAX);
    let path: Vec<_> = chain[cut..]
        .iter()
        .map(|n| json!({"id":n.id,"kind":n.kind,"name":n.props["name"]}))
        .collect();
    let mut value = json!({"id":node.id,"kind":node.kind,"name":node.props["name"],"parentId":node.parent_id,"depth":depth,"path":path,"matchedOn":matched_on});
    if cut > 0 {
        value["pathTruncated"] = json!(true);
    }
    if let (Some(needle), Some(text)) = (&criteria.text, node.props["text"].as_str()) {
        value["textPreview"] = json!(preview(text, needle));
    }
    if let Some(keys) = &req.properties {
        let mut props = node.props.clone();
        if let Some(object) = props.as_object_mut() {
            object.retain(|k, _| keys.contains(k));
        }
        value["props"] = props;
    }
    value
}

/// At most PREVIEW_CHARS characters (never bytes) of the text, positioned around the first match.
fn preview(text: &str, needle: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= PREVIEW_CHARS {
        return text.to_owned();
    }
    let lower = text.to_lowercase();
    let at = lower.find(needle).map_or(0, |b| lower[..b].chars().count());
    let start = at
        .saturating_sub(PREVIEW_CHARS / 4)
        .min(chars.len() - PREVIEW_CHARS);
    chars[start..start + PREVIEW_CHARS].iter().collect()
}

/// Criteria as requested; ids are echoed as a count so a 200-ID lookup does not repeat on
/// every page.
pub(super) fn echo(req: &ReadRequest) -> Value {
    let mut out = json!({"kinds":req.kinds,"nameContains":req.name_contains,"textContains":req.text_contains,"idCount":req.ids.as_ref().map(Vec::len),"parentId":req.parent_id});
    if let Some(fields) = out.as_object_mut() {
        fields.retain(|_, v| !v.is_null());
    }
    out
}
