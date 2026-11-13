//! Authored/resolved node entries; pagination and frame metadata are separate.
use super::{compact, index::TreeIndex, instances, viewports::Computed, NodeFormat, ReadRequest};
use crate::node_rows;
use opencanvas_storage::Node;
use serde_json::{json, Value};
use std::collections::HashMap;

pub(super) struct Source<'a> {
    pub index: &'a TreeIndex<'a>,
    pub expanded: &'a TreeIndex<'a>,
    pub resolved: &'a HashMap<&'a str, &'a Node>,
    pub computed: Option<&'a Computed>,
}

impl Source<'_> {
    pub fn entries(
        &self,
        name: &str,
        req: &ReadRequest,
        ordered: &[(&Node, usize)],
        compact: bool,
    ) -> Vec<(String, Value)> {
        let context = name == "get_design_context";
        let rows = context && matches!(req.node_format, Some(NodeFormat::Rows));
        let expand = context && req.instances.as_deref() == Some("expand");
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
                    let mut props = self.resolved[node.id.as_str()].props.clone();
                    value["effectiveTheme"] = props
                        .as_object_mut()
                        .unwrap()
                        .remove("effectiveTheme")
                        .unwrap();
                    if super::states::requested(req) {
                        if let Some(states) = super::states::differing(&props, &node.kind) {
                            value["resolvedStates"] = states;
                        }
                    }
                    if let Some(filter) = &req.properties {
                        props
                            .as_object_mut()
                            .unwrap()
                            .retain(|k, _| filter.contains(k));
                    }
                    value["resolvedProps"] =
                        compact::resolved_props(compact, &authored.props, props);
                    if let Some(computed) = &self.computed {
                        value["computedBounds"] = json!(computed.bounds.get(&node.id));
                    }
                }
                value
            };
            if !expand {
                instances::reference(node, &mut value, self.expanded, self.computed);
            }
            let omitted = self.index.child_count(&node.id);
            if req.depth == Some(*d) && omitted > 0 {
                value["omittedChildren"] = json!(omitted);
            }
            if rows {
                value = node_rows::pack(value);
            }
            entries.push(("nodes".into(), value));
        }

        entries
    }
}
