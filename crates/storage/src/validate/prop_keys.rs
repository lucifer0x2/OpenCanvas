//! Key-level checks of authored properties against the shared property catalog.
use crate::{
    error::*,
    model::*,
    props_catalog::{allowed_props, prop_spec, required_props, PropScope},
    props_suggest::suggest_prop,
};
use serde_json::{Map, Value};

/// Every resolved key must exist in the catalog and be allowed on this node kind.
pub(super) fn check_keys(kind: &NodeKind, props: &Map<String, Value>) -> Result<()> {
    for key in props.keys() {
        match prop_spec(key) {
            None => {
                let suggestion = suggest_prop(key);
                let hint = suggestion
                    .map(|s| format!("; did you mean '{s}'?"))
                    .unwrap_or_default();
                let message = format!(
                    "Unsupported authored property '{key}' on {} node{hint}",
                    kind.as_str()
                );
                let mut e = key_error(kind, key, &message);
                e.node_context().suggestion = suggestion.map(Into::into);
                return Err(e);
            }
            Some(spec) if !spec.scope.allows(kind) => {
                let message = format!(
                    "{} ('{key}' on {} node)",
                    spec.scope.requirement(),
                    kind.as_str()
                );
                return Err(key_error(kind, key, &message));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// Required keys (text content and metrics, image asset) must be present.
pub(super) fn check_required(kind: &NodeKind, props: &Map<String, Value>) -> Result<()> {
    for key in required_props(kind) {
        if !props.contains_key(key) {
            let message = match prop_spec(key).map(|spec| spec.scope) {
                Some(PropScope::Image) => "Image requires an existing asset",
                _ => {
                    "Text requires explicit text, fontId, fontSize and lineHeight (or a text style)"
                }
            };
            return Err(StorageError::validation(key, message));
        }
    }
    Ok(())
}

fn key_error(kind: &NodeKind, key: &str, message: &str) -> StorageError {
    let mut e = StorageError::validation(&format!("props.{key}"), message);
    let node = e.node_context();
    node.node_kind = Some(kind.as_str().into());
    node.allowed_keys = Some(allowed_props(kind).into_iter().map(Into::into).collect());
    e
}
