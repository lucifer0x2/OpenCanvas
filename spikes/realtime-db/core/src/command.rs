use crate::{DomainError, Node};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(tag = "op", deny_unknown_fields)]
pub(crate) enum Operation {
    #[serde(rename = "create_node", rename_all = "camelCase")]
    Create {
        id: String,
        parent_id: Option<String>,
        kind: String,
        props: Value,
        #[serde(default)]
        sort_key: i64,
    },
    #[serde(rename = "update_node")]
    Update { id: String, props: Value },
    #[serde(rename = "reparent_node", rename_all = "camelCase")]
    Reparent {
        id: String,
        parent_id: Option<String>,
        sort_key: Option<i64>,
    },
    #[serde(rename = "delete_subtree")]
    Delete { id: String },
}

pub fn execute(nodes: &mut Vec<Node>, operations: &[Value]) -> Result<Vec<String>, DomainError> {
    let mut changed = BTreeSet::new();
    for (index, value) in operations.iter().enumerate() {
        let op: Operation = serde_json::from_value(value.clone()).map_err(|e| {
            DomainError::new("VALIDATION_FAILED", format!("Operation {index}: {e}"))
        })?;
        match op {
            Operation::Create {
                id,
                parent_id,
                kind,
                props,
                sort_key,
            } => {
                if nodes.iter().any(|n| n.id == id) {
                    return Err(error("Duplicate node ID"));
                }
                changed.insert(id.clone());
                nodes.push(Node {
                    id,
                    parent_id,
                    kind,
                    props,
                    sort_key,
                });
            }
            Operation::Update { id, props } => {
                let node = find(nodes, &id)?;
                let patch = props
                    .as_object()
                    .ok_or_else(|| error("Props must be an object"))?;
                let target = node
                    .props
                    .as_object_mut()
                    .ok_or_else(|| error("Props must be an object"))?;
                target.extend(patch.clone());
                changed.insert(id);
            }
            Operation::Reparent {
                id,
                parent_id,
                sort_key,
            } => {
                let node = find(nodes, &id)?;
                node.parent_id = parent_id;
                if let Some(key) = sort_key {
                    node.sort_key = key;
                }
                changed.insert(id);
            }
            Operation::Delete { id } => {
                find(nodes, &id)?;
                let mut removed = BTreeSet::from([id]);
                loop {
                    let before = removed.len();
                    for node in nodes.iter() {
                        if node.parent_id.as_ref().is_some_and(|p| removed.contains(p)) {
                            removed.insert(node.id.clone());
                        }
                    }
                    if removed.len() == before {
                        break;
                    }
                }
                nodes.retain(|n| !removed.contains(&n.id));
                changed.extend(removed);
            }
        }
    }
    Ok(changed.into_iter().collect())
}

fn find<'a>(nodes: &'a mut [Node], id: &str) -> Result<&'a mut Node, DomainError> {
    nodes
        .iter_mut()
        .find(|n| n.id == id)
        .ok_or_else(|| error("Node does not exist"))
}

fn error(message: &str) -> DomainError {
    DomainError::new("VALIDATION_FAILED", message)
}
