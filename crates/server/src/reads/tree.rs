use super::{pagination, sanitize_asset, ReadRequest};
use crate::error::{ApiError, Result};
use opencanvas_storage::{
    resolved_nodes_for_theme, resolved_tokens_for_theme, Node, NodeKind, Snapshot,
};
use serde_json::{json, Value};

pub(super) fn read(
    s: &Snapshot,
    name: &str,
    req: ReadRequest,
    scope: String,
    offset: usize,
) -> Result<Value> {
    let root = req.node_id.as_ref().or(req.frame_id.as_ref());
    if (name == "get_subtree" && req.node_id.is_none())
        || (name == "get_design_context" && req.frame_id.is_none())
    {
        return Err(ApiError::invalid(
            "A subtree requires nodeId; context requires frameId",
        ));
    }
    if req.node_id.is_some() && req.frame_id.is_some() {
        return Err(ApiError::invalid("Choose one scope ID"));
    }
    if let Some(id) = root {
        let node = s
            .nodes
            .iter()
            .find(|n| &n.id == id)
            .ok_or_else(|| ApiError::invalid("Scope node does not exist"))?;
        if name == "get_design_context" && node.kind != NodeKind::Frame {
            return Err(ApiError::invalid("frameId must identify a frame"));
        }
    }
    let context = name == "get_design_context";
    let theme = req.theme.as_deref().unwrap_or("default");
    let resolved = if context {
        resolved_nodes_for_theme(s, theme)?
    } else {
        Vec::new()
    };
    let mut ordered = Vec::new();
    let mut roots: Vec<_> = s
        .nodes
        .iter()
        .filter(|n| root.map_or(n.parent_id.is_none(), |id| &n.id == id))
        .collect();
    roots.sort_by_key(|n| n.sort_key);
    for n in roots {
        visit(&s.nodes, n, 0, &mut ordered);
    }
    let mut entries = Vec::new();
    for (node, d) in ordered {
        let value = if name == "get_outline" {
            json!({"id":node.id,"parentId":node.parent_id,"kind":node.kind,"name":node.props["name"],"sortKey":node.sort_key})
        } else {
            let mut authored = node.clone();
            if let Some(props) = &req.properties {
                authored
                    .props
                    .as_object_mut()
                    .unwrap()
                    .retain(|k, _| props.contains(k));
            }
            let mut value = json!(authored);
            if context {
                let mut props = resolved
                    .iter()
                    .find(|n| n.id == node.id)
                    .unwrap()
                    .props
                    .clone();
                if let Some(filter) = &req.properties {
                    props
                        .as_object_mut()
                        .unwrap()
                        .retain(|k, _| filter.contains(k));
                }
                value["resolvedProps"] = props;
            }
            value
        };
        entries.push(("nodes".into(), value, d));
    }
    let mut response = json!({"schemaVersion":1,"projectId":s.project_id,"revision":s.revision,"scope":root,"nodes":[],"limits":{"depth":req.depth.unwrap_or(8),"nodes":req.limit.unwrap_or(200),"bytes":req.budget.unwrap_or(65536)}});
    if context {
        response["theme"] = json!(theme);
        response["viewport"] = json!(req.viewport);
        response["diagnostics"] = json!([]);
        let tokens = resolved_tokens_for_theme(s, theme)?;
        let value = serde_json::to_value(s)?;
        for field in ["tokens", "styles", "assets", "fonts"] {
            response[field] = json!([]);
            for item in value[field].as_array().unwrap() {
                let mut item = sanitize_asset(item.clone());
                if field == "tokens" {
                    item["resolvedValue"] = tokens[item["id"].as_str().unwrap()].clone();
                }
                if field == "styles" {
                    item["resolvedValue"] = resolve_style(&item["value"], &tokens);
                }
                entries.push((field.into(), item, 0));
            }
        }
    }
    pagination::page(
        response,
        entries,
        offset,
        req.limit.unwrap_or(200),
        req.budget.unwrap_or(65536),
        req.depth.unwrap_or(8),
        &scope,
    )
}
fn resolve_style(value: &Value, tokens: &std::collections::BTreeMap<String, Value>) -> Value {
    if let Some(id) = value["tokenId"].as_str() {
        return tokens.get(id).cloned().unwrap_or(Value::Null);
    }
    match value {
        Value::Object(fields) => fields
            .iter()
            .map(|(k, v)| (k.clone(), resolve_style(v, tokens)))
            .collect(),
        _ => value.clone(),
    }
}
fn visit<'a>(all: &'a [Node], node: &'a Node, depth: usize, out: &mut Vec<(&'a Node, usize)>) {
    out.push((node, depth));
    let mut children: Vec<_> = all
        .iter()
        .filter(|n| n.parent_id.as_deref() == Some(&node.id))
        .collect();
    children.sort_by_key(|n| n.sort_key);
    for child in children {
        visit(all, child, depth + 1, out);
    }
}
