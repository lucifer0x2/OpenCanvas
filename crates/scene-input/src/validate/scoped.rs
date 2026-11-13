//! Resolution after the caller's explicit full-snapshot validation boundary.
use super::Candidate;
use crate::{Node, Result, Snapshot};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn resources(snapshot: &Snapshot) -> Candidate {
    Candidate {
        theme: "default".into(),
        nodes: BTreeMap::new(),
        tokens: snapshot
            .tokens
            .iter()
            .cloned()
            .map(|n| (n.id.clone(), n))
            .collect(),
        styles: snapshot
            .styles
            .iter()
            .cloned()
            .map(|n| (n.id.clone(), n))
            .collect(),
        assets: snapshot
            .assets
            .iter()
            .cloned()
            .map(|n| (n.id.clone(), n))
            .collect(),
        fonts: snapshot
            .fonts
            .iter()
            .cloned()
            .map(|n| (n.id.clone(), n))
            .collect(),
        notes: BTreeMap::new(),
        deleted: Default::default(),
    }
}

pub(crate) fn check_scoped_theme(snapshot: &Snapshot, theme: &str) -> Result<()> {
    resources(snapshot).select_theme(theme)
}

pub(crate) fn resolve_scoped(
    snapshot: &Snapshot,
    nodes: &[Node],
    theme: &str,
) -> Result<Vec<Node>> {
    let mut candidate = resources(snapshot);
    candidate.select_theme(theme)?;
    candidate.nodes = nodes.iter().cloned().map(|n| (n.id.clone(), n)).collect();
    nodes
        .iter()
        .cloned()
        .map(|mut node| {
            let effective = candidate.effective_theme(&node)?.to_owned();
            node.props = candidate.resolve_props(&node)?;
            node.props["effectiveTheme"] = Value::String(effective);
            if node.props["visible"] == false {
                node.props["opacity"] = json!(0);
            }
            Ok(node)
        })
        .collect()
}

pub(crate) fn resolve_scoped_tokens(
    snapshot: &Snapshot,
    theme: &str,
) -> Result<BTreeMap<String, Value>> {
    let mut candidate = resources(snapshot);
    candidate.select_theme(theme)?;
    candidate
        .tokens
        .keys()
        .map(|id| Ok((id.clone(), candidate.token_value(id, theme)?)))
        .collect()
}
