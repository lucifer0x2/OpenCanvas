use super::{
    animated, compact, diagnostics, index::TreeIndex, instances, notes, pagination, tree_resources,
    viewports::Computed, NodeFormat, ReadRequest,
};
use crate::error::{ApiError, Result};
use crate::node_rows;
use opencanvas_storage::{
    read_scope::{ReadScope, ScopedSnapshot},
    NodeKind, SharedSnapshot,
};
use serde_json::{json, Value};
use std::collections::HashMap;

// Frozen full implementation: custom backend compatibility and differential tests.
// A project-wide outline intentionally remains a full read.
#[cfg(test)]
pub(super) use super::tree_legacy::read;

pub(super) fn read_shared(
    shared: &SharedSnapshot,
    name: &str,
    req: ReadRequest,
    scope: String,
    offset: usize,
    computed: Option<Computed>,
) -> Result<Value> {
    if let Some(scoped) = shared.trusted_scoped()? {
        if name != "get_outline" || req.node_id.is_some() || req.frame_id.is_some() {
            return read_scoped(&scoped, name, req, scope, offset, computed);
        }
    }
    super::tree_legacy::read(&shared.snapshot, name, req, scope, offset, computed)
}

pub(super) fn read_scoped(
    scoped: &ScopedSnapshot<'_>,
    name: &str,
    req: ReadRequest,
    scope: String,
    offset: usize,
    computed: Option<Computed>,
) -> Result<Value> {
    let s = scoped.snapshot();
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
    let context = name == "get_design_context";
    let rows = context && matches!(req.node_format, Some(NodeFormat::Rows));
    let root = root.expect("scoped reads require a target");
    let mut nodes = scoped.expand(ReadScope::Target(root), context)?;
    let expand = context && req.instances.as_deref() == Some("expand");
    let use_expanded = expand || root.contains('/');
    let root_node = if use_expanded {
        nodes.expanded.iter().find(|n| n.id == *root)
    } else {
        nodes.authored.iter().copied().find(|n| n.id == *root)
    }
    .ok_or_else(|| ApiError::invalid("Scope node does not exist"))?;
    if context && root_node.kind != NodeKind::Frame {
        return Err(ApiError::invalid("frameId must identify a frame"));
    }
    let theme = req.theme.as_deref().unwrap_or("default");
    let resolved = if context {
        scoped.resolve(&mut nodes, theme)?
    } else {
        Vec::new()
    };
    let expanded = &nodes.expanded;
    let authored_index = TreeIndex::from_nodes(nodes.authored.iter().copied());
    let expanded_index = TreeIndex::new(expanded);
    let index = if use_expanded {
        &expanded_index
    } else {
        &authored_index
    };
    let by_id: HashMap<_, _> = resolved.iter().map(|n| (n.id.as_str(), n)).collect();
    let ordered = index.preorder(Some(root.as_str()));
    let expanded_ordered =
        (context && !use_expanded).then(|| expanded_index.preorder(Some(root.as_str())));
    let full_scope = expanded_ordered.as_deref().unwrap_or(&ordered);
    let scope_notes = if context {
        scoped.notes_for_nodes(nodes.dependencies.iter().map(|n| n.id.as_str()))
    } else {
        Vec::new()
    };
    let compact = (context && req.compact == Some(true))
        .then(|| {
            compact::Referenced::collect_scoped(
                s,
                full_scope
                    .iter()
                    .map(|(n, _)| *n)
                    .chain(instances::definition_nodes_indexed(
                        &authored_index,
                        &ordered,
                    )),
                full_scope
                    .iter()
                    .filter_map(|(n, _)| by_id.get(n.id.as_str()))
                    .filter_map(|n| n.props["effectiveTheme"].as_str())
                    .chain(std::iter::once(theme)),
                expanded,
            )
        })
        .transpose()?;
    let mut entries = super::tree_nodes::Source {
        index,
        expanded: &expanded_index,
        resolved: &by_id,
        computed: computed.as_ref(),
    }
    .entries(name, &req, &ordered, compact.is_some());
    if context {
        entries.extend(notes::entries_indexed(
            scope_notes.iter().copied(),
            index,
            root,
            &ordered,
            req.notes,
        ));
    }
    let mut response = json!({"schemaVersion":1,"projectId":s.project_id,"revision":s.revision,"scope":root,"nodes":[],"limits":{"depth":req.depth,"nodes":req.limit.unwrap_or(200),"bytes":req.budget.unwrap_or(65536)}});
    if rows {
        response["nodeFormat"] = json!(node_rows::FORMAT);
        response["nodeColumns"] = json!(node_rows::COLUMNS);
    }
    if context {
        if !expand && offset == 0 {
            let definitions = instances::definitions_indexed(
                &authored_index,
                &ordered,
                &by_id,
                compact.is_some(),
                super::states::requested(&req),
            );
            if definitions.as_object().is_some_and(|o| !o.is_empty()) {
                response["components"] = definitions;
            }
        }
        response["theme"] = json!(theme);
        compact::mark(&mut response, compact.as_ref());
        response["viewport"] = json!(req.viewport);
        response["diagnostics"] = diagnostics::for_scope(full_scope);
        let text_diagnostics = crate::text_diagnostics::authored_scoped(
            &nodes.dependencies,
            &scope_notes,
            &resolved,
            root,
        );
        // Navigation is frame metadata: on the first page only, absent when the frame has none.
        if offset == 0 {
            if let Some(nav) = super::tree_navigation::summary(scoped, &nodes, root) {
                response["navigation"] = nav;
            }
        }
        if let Some(computed) = computed {
            let measured_diagnostics = computed.diagnostics;
            response["computed"] = computed.metadata;
            response["reflow"] = json!(computed.reflow);
            if computed.reflow == Some(true) {
                response["diagnostics"] = json!([]);
            } else if computed.reflow == Some(false) && response["diagnostics"] == json!([]) {
                response["diagnostics"] = diagnostics::unchanged(full_scope);
            }
            response["diagnostics"]
                .as_array_mut()
                .unwrap()
                .extend(measured_diagnostics);
        }
        response["diagnostics"]
            .as_array_mut()
            .unwrap()
            .extend(text_diagnostics);
        for field in ["tokens", "styles", "assets", "fonts"] {
            response[field] = json!([]);
        }
        entries.extend(tree_resources::entries(
            s,
            &scoped.resolved_tokens(theme)?,
            compact.as_ref(),
        )?);
        if animated::has_tracks(&ordered) {
            let budget = req.budget.unwrap_or(65536);
            let headroom = pagination::metadata_headroom(
                &response,
                entries.get(offset),
                offset,
                budget,
                &scope,
            )?;
            animated::include(&mut response, &ordered, headroom.min(budget / 4))?;
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
