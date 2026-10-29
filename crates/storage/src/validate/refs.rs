use super::{values::*, Candidate};
use crate::{error::*, model::*};
use serde_json::{Map, Value};
impl Candidate {
    pub fn resolve_value(&self, v: &Value, expected: &str) -> Result<Value> {
        if let Some(o) = v.as_object() {
            if o.len() != 1 {
                return Err(StorageError::validation(
                    "ref",
                    "A typed reference must contain exactly one ID",
                ));
            }
            if let Some(t) = o.get("tokenId").and_then(Value::as_str) {
                let token = self.tokens.get(t).ok_or_else(|| {
                    StorageError::validation(
                        "tokenId",
                        "Referenced token does not exist; forward references are not supported",
                    )
                })?;
                if token.token_type != expected {
                    return Err(StorageError::validation(
                        "tokenId",
                        "Token type does not match this property",
                    ));
                }
                return self.token_value(&token.id, &self.theme);
            }
            if expected == "color" {
                if let Some(s) = o.get("styleId").and_then(Value::as_str) {
                    let style = self.styles.get(s).ok_or_else(|| {
                        StorageError::validation("styleId", "Referenced style does not exist")
                    })?;
                    if style.kind != "fill" {
                        return Err(StorageError::validation(
                            "styleId",
                            "A fill property requires a fill style",
                        ));
                    }
                    // Defend the resolver itself as well as authored validation:
                    // even a legacy/corrupted style row cannot recurse through
                    // style-to-style aliases or overflow the process stack.
                    if style.value.get("styleId").is_some() {
                        return Err(StorageError::validation(
                            "style.value",
                            "Style aliases are not supported",
                        ));
                    }
                    return self.resolve_value(&style.value, "color");
                }
            }
            return Err(StorageError::validation(
                "ref",
                "Unsupported reference binding",
            ));
        }
        literal(v, expected)?;
        Ok(v.clone())
    }
    pub fn resolve_props(&self, node: &Node) -> Result<Value> {
        let authored = node
            .props
            .as_object()
            .ok_or_else(|| StorageError::validation("props", "Properties must be an object"))?;
        let mut out = Map::new();
        if let Some(style_id) = authored.get("styleId") {
            let style = self
                .styles
                .get(style_id.as_str().unwrap_or(""))
                .ok_or_else(|| {
                    StorageError::validation("styleId", "Referenced style does not exist")
                })?;
            match style.kind.as_str() {
                "text" if node.kind == NodeKind::Text => out.extend(
                    style
                        .value
                        .as_object()
                        .ok_or_else(|| {
                            StorageError::validation("style.value", "Text style must be an object")
                        })?
                        .clone(),
                ),
                "fill" => {
                    out.insert("fill".into(), style.value.clone());
                }
                _ => {
                    return Err(StorageError::validation(
                        "styleId",
                        "Style kind is not supported on this node",
                    ))
                }
            }
        }
        out.extend(authored.clone());
        out.remove("styleId");
        for (k, v) in out.iter_mut() {
            let expected = match k.as_str() {
                "fill" => Some("color"),
                "width" | "height" | "minWidth" | "minHeight" | "maxWidth" | "maxHeight"
                | "fontSize" | "lineHeight" | "radius" => Some("dimension"),
                "x" | "y" | "opacity" => Some("number"),
                _ => None,
            };
            if let Some(expected) = expected {
                if v.is_object() {
                    *v = self.resolve_value(v, expected)?;
                }
            }
            if k == "stroke" {
                self.resolve_stroke(v)?;
            }
            if k == "animations" {
                self.resolve_animations(v)?;
            }
            if k == "states" && node.kind != NodeKind::Page {
                self.resolve_states(&node.kind, v)?;
            }
            if k == "layout" {
                self.resolve_layout_refs(v)?;
            }
        }
        Ok(Value::Object(out))
    }
}
/// Resolve only supported typed refs into primitive renderer properties.
pub fn resolved_nodes(snapshot: &Snapshot) -> Result<Vec<Node>> {
    resolved_nodes_for_theme(snapshot, "default")
}
/// Resolve a concrete design theme; authored snapshot properties stay intact.
pub fn resolved_nodes_for_theme(snapshot: &Snapshot, theme: &str) -> Result<Vec<Node>> {
    let mut c = Candidate::from_snapshot(snapshot);
    c.validate()?;
    c.select_theme(theme)?;
    snapshot
        .nodes
        .iter()
        .map(|n| {
            let mut n = n.clone();
            // `fit` keeps its authored name: resolved props and scenes share IMAGE_FITS.
            n.props = c.resolve_props(&n)?;
            Ok(n)
        })
        .collect()
}

/// Read back literal token values at one concrete theme and snapshot revision.
pub fn resolved_tokens_for_theme(
    snapshot: &Snapshot,
    theme: &str,
) -> Result<std::collections::BTreeMap<String, Value>> {
    let mut c = Candidate::from_snapshot(snapshot);
    c.validate()?;
    c.select_theme(theme)?;
    c.tokens
        .keys()
        .map(|id| Ok((id.clone(), c.token_value(id, theme)?)))
        .collect()
}
