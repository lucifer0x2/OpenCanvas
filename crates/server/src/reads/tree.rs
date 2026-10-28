use super::{
    animated, compact, diagnostics, index::TreeIndex, navigation, notes, pagination,
    sanitize_asset,
    viewports::Computed, ReadRequest,
};
use crate::error::{ApiError, Result};
use opencanvas_storage::{resolved_nodes_for_theme, resolved_tokens_for_theme, NodeKind, Snapshot};
use serde_json::{json, Value};
use std::collections::HashMap;

pub(super) fn read(
    s: &Snapshot,
    name: &str,
    req: ReadRequest,
    scope: String,
    offset: usize,
    computed: Option<Computed>,
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
    let index = TreeIndex::new(&s.nodes);
    if let Some(id) = root {
        let node = index
            .get(id)
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
    let by_id: HashMap<_, _> = resolved.iter().map(|n| (n.id.as_str(), n)).collect();
    let ordered = index.preorder(root.map(String::as_str));
    let compact = (context && req.compact == Some(true))
        .then(|| compact::Referenced::collect(s, ordered.iter().map(|(n, _)| *n)));
    let mut entries = Vec::new();
    // depth filters from the scope root (depth 0); omitted, it lists the whole scope.
    for (node, d) in ordered
        .iter()
        .filter(|(_, d)| req.depth.is_none_or(|max| *d <= max))
    {
        let mut value = if name == "get_outline" {
            json!({"id":node.id,"parentId":node.parent_id,"kind":node.kind,"name":node.props["name"],"sortKey":node.sort_key})
        } else {
            let mut authored = (*node).clone();
            if let Some(props) = &req.properties {
                authored
                    .props
                    .as_object_mut()
                    .unwrap()
                    .retain(|k, _| props.contains(k));
            }
            let mut value = json!(authored);
            if context {
                let mut props = by_id[node.id.as_str()].props.clone();
                if let Some(filter) = &req.properties {
                    props
                        .as_object_mut()
                        .unwrap()
                        .retain(|k, _| filter.contains(k));
                }
                value["resolvedProps"] =
                    compact::resolved_props(compact.is_some(), &authored.props, props);
                if let Some(computed) = &computed {
                    value["computedBounds"] = json!(computed.bounds[&node.id]);
                }
            }
            value
        };
        let omitted = index.child_count(&node.id);
        if req.depth == Some(*d) && omitted > 0 {
            value["omittedChildren"] = json!(omitted);
        }
        entries.push(("nodes".into(), value));
    }
    if let (true, Some(frame)) = (context, root) {
        entries.extend(notes::entries(s, &index, frame, &ordered, req.notes));
    }
    let mut response = json!({"schemaVersion":1,"projectId":s.project_id,"revision":s.revision,"scope":root,"nodes":[],"limits":{"depth":req.depth,"nodes":req.limit.unwrap_or(200),"bytes":req.budget.unwrap_or(65536)}});
    if context {
        response["theme"] = json!(theme);
        compact::mark(&mut response, compact.as_ref());
        response["viewport"] = json!(req.viewport);
        response["diagnostics"] = diagnostics::for_scope(&ordered);
        // Navigation is frame metadata: on the first page only, absent when the frame has none.
        if let Some(nav) = root
            .filter(|_| offset == 0)
            .and_then(|f| navigation::summary(&index, f))
        {
            response["navigation"] = nav;
        }
        if let Some(animated) = animated::summary(&ordered) {
            response["animated"] = animated;
        }
        if let Some(computed) = computed {
            response["computed"] = computed.metadata;
            response["reflow"] = json!(computed.reflow);
            if computed.reflow == Some(true) {
                response["diagnostics"] = json!([]);
            } else if computed.reflow == Some(false) && response["diagnostics"] == json!([]) {
                response["diagnostics"] = diagnostics::unchanged(ordered[0].0);
            }
        }
        let tokens = resolved_tokens_for_theme(s, theme)?;
        let value = serde_json::to_value(s)?;
        for field in ["tokens", "styles", "assets", "fonts"] {
            response[field] = json!([]);
            for item in compact::kept(&value[field], field, compact.as_ref()) {
                let mut item = sanitize_asset(item.clone());
                if field == "tokens" {
                    item["resolvedValue"] = tokens[item["id"].as_str().unwrap()].clone();
                }
                if field == "styles" {
                    item["resolvedValue"] = resolve_style(&item["value"], &tokens);
                }
                entries.push((field.into(), item));
            }
        }
    }
    pagination::page(
        response,
        entries,
        offset,
        req.limit.unwrap_or(200),
        req.budget.unwrap_or(65536),
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
