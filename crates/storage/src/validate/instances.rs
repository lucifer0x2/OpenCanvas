//! Component graph, override shape and authored/virtual write boundaries.
use super::Candidate;
use crate::{
    instances::{expand, INSTANCE_KEYS, OVERRIDE_KEYS},
    *,
};

impl Candidate {
    pub(super) fn validate_instances(&self) -> Result<()> {
        if !self
            .nodes
            .values()
            .any(|n| n.kind == NodeKind::Instance || n.props.get("component").is_some())
        {
            return Ok(());
        }
        let has_instances = self.nodes.values().any(|n| n.kind == NodeKind::Instance);
        self.instance_graph()?;
        for node in self.nodes.values() {
            if let Some(component) = node.props.get("component") {
                if component.as_object().is_none_or(|o| o.len() != 1)
                    || component["name"]
                        .as_str()
                        .is_none_or(|s| s.is_empty() || s.len() > 256)
                {
                    return Err(StorageError::validation(
                        "props.component",
                        "Component requires a name up to 256 bytes",
                    ));
                }
            }
            if node.kind == NodeKind::Instance {
                for key in node
                    .props
                    .as_object()
                    .ok_or_else(|| {
                        StorageError::validation("props", "Instance properties must be an object")
                    })?
                    .keys()
                {
                    if !INSTANCE_KEYS.contains(&key.as_str()) {
                        return Err(key_error(&format!("props.{key}"), INSTANCE_KEYS));
                    }
                }
                let master = node.props["componentId"]
                    .as_str()
                    .and_then(|id| self.nodes.get(id))
                    .filter(|n| n.kind == NodeKind::Frame && n.props.get("component").is_some())
                    .ok_or_else(|| {
                        StorageError::validation("props.componentId", "Expected a component frame")
                    })?;
                if let Some(value) = node.props.get("overrides") {
                    let overrides = value.as_object().ok_or_else(|| {
                        StorageError::validation("props.overrides", "Overrides must be an object")
                    })?;
                    for (target, patch) in overrides {
                        if target == &master.id || !self.ancestor_is(target, &master.id) {
                            return Err(StorageError::validation(
                                &format!("props.overrides.{target}"),
                                "Override target must be a master descendant",
                            ));
                        }
                        let patch = patch.as_object().ok_or_else(|| {
                            StorageError::validation(
                                &format!("props.overrides.{target}"),
                                "Override must be a property object",
                            )
                        })?;
                        for key in patch.keys() {
                            if !OVERRIDE_KEYS.contains(&key.as_str()) {
                                return Err(key_error(
                                    &format!("props.overrides.{target}.{key}"),
                                    OVERRIDE_KEYS,
                                ));
                            }
                        }
                        let mut child = self.nodes[target].clone();
                        child.props.as_object_mut().unwrap().extend(patch.clone());
                        self.properties(&child).map_err(|mut e| {
                            e.field = Some(format!(
                                "props.overrides.{target}.{}",
                                e.field
                                    .unwrap_or_else(|| "props".into())
                                    .trim_start_matches("props.")
                            ));
                            e
                        })?;
                    }
                }
            }
            if self.in_component(&node.id)
                && ["links", "animations", "flowStart"].iter().any(|key| {
                    node.props
                        .get(key)
                        .is_some_and(|v| v.as_array().is_none_or(|a| !a.is_empty()))
                })
            {
                return Err(StorageError::validation(
                    "props.component",
                    "Component subtrees cannot carry links, flowStart or animations",
                ));
            }
        }
        if has_instances {
            let nodes: Vec<_> = self.nodes.values().cloned().collect();
            // The bounded expander rejects direct/indirect cycles and depth explosions.
            let expanded = expand(&nodes)?;
            for theme in self.validate_tokens()? {
                let mut placed = self.clone();
                placed.theme = theme;
                placed.nodes = expanded
                    .iter()
                    .cloned()
                    .map(|n| (n.id.clone(), n))
                    .collect();
                for node in expanded.iter().filter(|n| {
                    n.id.contains('/')
                        || self
                            .nodes
                            .get(&n.id)
                            .is_some_and(|n| n.kind == NodeKind::Instance)
                }) {
                    placed.properties(node)?;
                }
            }
        }
        Ok(())
    }

    fn ancestor_is(&self, id: &str, ancestor: &str) -> bool {
        let mut current = self.nodes.get(id).and_then(|n| n.parent_id.as_deref());
        for _ in 0..128 {
            let Some(id) = current else {
                return false;
            };
            if id == ancestor {
                return true;
            }
            current = self.nodes.get(id).and_then(|n| n.parent_id.as_deref());
        }
        false
    }
    fn in_component(&self, id: &str) -> bool {
        let mut current = self.nodes.get(id);
        for _ in 0..128 {
            let Some(node) = current else {
                break;
            };
            if node.props.get("component").is_some() {
                return true;
            }
            current = node.parent_id.as_ref().and_then(|id| self.nodes.get(id));
        }
        false
    }
    pub(super) fn instance_write_guard(&self, op: &Operation) -> Result<()> {
        let parent = match op {
            Operation::CreateNode { parent_id, .. } => parent_id.as_deref(),
            Operation::ReparentNode { parent_id, .. } => Some(parent_id.as_str()),
            _ => None,
        };
        let id = match op {
            Operation::CreateNode { id, .. }
            | Operation::UpdateNode { id, .. }
            | Operation::ReparentNode { id, .. }
            | Operation::ReorderNode { id, .. }
            | Operation::DeleteSubtree { id } => id,
            _ => return Ok(()),
        };
        if let Some((instance, _)) = id
            .split_once('/')
            .or_else(|| parent.and_then(|id| id.split_once('/')))
        {
            return Err(StorageError::new(
                "INSTANCE_CHILD_READONLY",
                &format!("Virtual children are read-only; update_node {instance} props.overrides"),
            ));
        }
        Ok(())
    }
    pub(super) fn component_delete_guard(&self, remove: &BTreeSet<String>) -> Result<()> {
        let users: Vec<_> = self
            .nodes
            .values()
            .filter(|n| {
                !remove.contains(&n.id)
                    && n.kind == NodeKind::Instance
                    && n.props["componentId"]
                        .as_str()
                        .is_some_and(|id| remove.contains(id))
            })
            .map(|n| n.id.clone())
            .collect();
        if !users.is_empty() {
            let mut error = StorageError::new(
                "COMPONENT_IN_USE",
                "Delete dependent instances before deleting their component",
            );
            error.node_context().instance_ids = Some(users.iter().take(20).cloned().collect());
            error.node_context().count = Some(users.len());
            return Err(error);
        }
        Ok(())
    }
}
fn key_error(field: &str, allowed: &[&str]) -> StorageError {
    let mut error = StorageError::validation(field, "Unsupported instance override or property");
    error.node_context().allowed_keys = Some(allowed.iter().map(|s| (*s).into()).collect());
    error
}
