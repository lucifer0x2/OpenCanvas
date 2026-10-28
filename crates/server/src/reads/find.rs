//! `find_nodes`: a flat preorder list of the nodes in one scope that match every requested
//! criterion, each carrying its ancestor path so it can be chosen without an outline.
use super::{index::TreeIndex, pagination, ReadRequest};
use crate::error::{ApiError, Result};
use opencanvas_storage::{Node, NodeKind, Snapshot};
use serde_json::{json, Value};
use std::collections::HashSet;

const PATH_MAX: usize = 8;
const PREVIEW_CHARS: usize = 120;
/// `pagination::page` caps every array except `nodes` at 100 items per page, so matches page
/// under `nodes` and are renamed afterwards; the rename lengthens the key by this many bytes.
const RENAME_BYTES: usize = "matches".len() - "nodes".len();

/// Requested criteria, ANDed. Substring needles are lowercased once (no accent folding).
struct Criteria<'r> {
    kinds: Option<&'r [String]>,
    name: Option<String>,
    text: Option<String>,
    ids: Option<HashSet<&'r str>>,
}

impl<'r> Criteria<'r> {
    fn new(req: &'r ReadRequest) -> Result<Self> {
        let criteria = Self {
            kinds: req.kinds.as_deref(),
            name: req.name_contains.as_deref().map(str::to_lowercase),
            text: req.text_contains.as_deref().map(str::to_lowercase),
            ids: req
                .ids
                .as_ref()
                .map(|ids| ids.iter().map(String::as_str).collect()),
        };
        if criteria.kinds.is_none()
            && criteria.name.is_none()
            && criteria.text.is_none()
            && criteria.ids.is_none()
        {
            return Err(ApiError::invalid(
                "find_nodes needs at least one of kinds, nameContains, textContains or ids",
            ));
        }
        Ok(criteria)
    }

    /// The criteria this node satisfies, or None when any requested criterion fails.
    fn matched_on(&self, node: &Node) -> Option<Vec<&'static str>> {
        let mut on = Vec::new();
        if let Some(needle) = &self.name {
            let name = node.props["name"].as_str()?;
            name.to_lowercase()
                .contains(needle.as_str())
                .then_some(())?;
            on.push("name");
        }
        if let Some(needle) = &self.text {
            let text = node.props["text"]
                .as_str()
                .filter(|_| node.kind == NodeKind::Text)?;
            text.to_lowercase()
                .contains(needle.as_str())
                .then_some(())?;
            on.push("text");
        }
        if let Some(kinds) = self.kinds {
            kinds
                .iter()
                .any(|k| k == node.kind.as_str())
                .then_some(())?;
            on.push("kind");
        }
        if let Some(ids) = &self.ids {
            ids.contains(node.id.as_str()).then_some(())?;
            on.push("id");
        }
        Some(on)
    }
}

pub(super) fn read(s: &Snapshot, req: ReadRequest) -> Result<Value> {
    let criteria = Criteria::new(&req)?;
    if req.node_id.is_some() && req.frame_id.is_some() {
        return Err(ApiError::invalid("Choose one scope ID"));
    }
    let (scope, offset) = pagination::scope(s, "find_nodes", &req)?;
    let index = TreeIndex::new(&s.nodes);
    let root = req.node_id.as_deref().or(req.frame_id.as_deref());
    if let Some(id) = root {
        let node = index
            .get(id)
            .ok_or_else(|| ApiError::invalid("Scope node does not exist"))?;
        if req.frame_id.is_some() && node.kind != NodeKind::Frame {
            return Err(ApiError::invalid("frameId must identify a frame"));
        }
    }
    let scanned = index.preorder(root);
    let entries: Vec<_> = scanned
        .iter()
        .filter_map(|(node, depth)| {
            let on = criteria.matched_on(node)?;
            let value = entry(&index, root, node, *depth, on, &criteria, &req);
            // Depth 0 on every entry: the flat list never stops a page on the depth rule.
            Some(("nodes".to_owned(), value, 0))
        })
        .collect();
    let limit = req.limit.unwrap_or(50);
    let budget = req.budget.unwrap_or(65536);
    let response = json!({"schemaVersion":1,"projectId":s.project_id,"revision":s.revision,"scope":root,"criteria":echo(&req),"scannedNodes":scanned.len(),"totalMatches":entries.len(),"nodes":[],"limits":{"nodes":limit,"bytes":budget}});
    let mut page = pagination::page(
        response,
        entries,
        offset,
        limit,
        budget - RENAME_BYTES,
        0,
        &scope,
    )?;
    let matches = page.as_object_mut().and_then(|o| o.remove("nodes"));
    page["matches"] = matches.unwrap_or_else(|| json!([]));
    Ok(page)
}

fn entry(
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
fn echo(req: &ReadRequest) -> Value {
    let mut out = json!({"kinds":req.kinds,"nameContains":req.name_contains,"textContains":req.text_contains,"idCount":req.ids.as_ref().map(Vec::len)});
    if let Some(fields) = out.as_object_mut() {
        fields.retain(|_, v| !v.is_null());
    }
    out
}
