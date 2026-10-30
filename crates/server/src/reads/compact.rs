//! Opt-in compact design context (`compact:true` on `get_design_context`).
//!
//! Two transforms, each declared on the response so a reader never has to guess:
//! - `resolvedPropsMode:"differing"`: a node's `resolvedProps` keeps only keys whose resolved
//!   value differs from the authored one (or that a style adds). The full resolved object is the
//!   authored props without `styleId`, overlaid with these keys.
//! - `resources:"referenced"`: tokens, styles, fonts and assets are only those the scope
//!   references, closed over style→token/font and token alias→token across every theme branch.
//!
//! Reference keys mirror storage validation, whose helpers are crate-private
//! (`crates/storage/src/validate/{refs,properties,node,tokens}.rs`): node and style props
//! reference tokens through `{tokenId}`, styles through `styleId` (top level or a fill
//! `{styleId}`), fonts through `fontId` and assets through `assetId`; token values reference
//! tokens through `{aliasId}` in the default branch or any `themes` branch. Storage rejects
//! these keys anywhere else, so a walk over every object key finds exactly that set.
//! `tests/compact_context.rs` pins the result against storage's own resolver.
use opencanvas_storage::{Node, Snapshot};
use serde_json::{json, Map, Value};
use std::collections::{BTreeSet, HashMap};

/// IDs of every resource one scope needs to resolve, including transitive references.
#[derive(Default)]
pub(super) struct Referenced {
    tokens: BTreeSet<String>,
    styles: BTreeSet<String>,
    fonts: BTreeSet<String>,
    assets: BTreeSet<String>,
}

impl Referenced {
    /// Collects over the full authored props of every scope node, independent of any
    /// `properties` filter or page, so each page of one read carries the same resource set.
    pub fn collect<'a>(
        s: &Snapshot,
        scope: impl IntoIterator<Item = &'a Node>,
    ) -> opencanvas_storage::Result<Self> {
        let mut refs = Self::default();
        let nodes: Vec<_> = scope.into_iter().collect();
        for node in &nodes {
            refs.walk(&node.props);
        }
        refs.fonts
            .extend(opencanvas_storage::referenced_font_ids(s, &nodes)?);
        // Storage rejects style-to-style aliases, so one pass over the styles is complete.
        let styles: HashMap<&str, &Value> =
            s.styles.iter().map(|x| (x.id.as_str(), &x.value)).collect();
        for id in refs.styles.clone() {
            if let Some(value) = styles.get(id.as_str()) {
                refs.walk(value);
            }
        }
        let tokens: HashMap<&str, &Value> =
            s.tokens.iter().map(|x| (x.id.as_str(), &x.value)).collect();
        let mut pending: Vec<String> = refs.tokens.iter().cloned().collect();
        while let Some(id) = pending.pop() {
            let Some(value) = tokens.get(id.as_str()) else {
                continue;
            };
            let mut found = Self::default();
            found.walk(value);
            for target in found.tokens {
                if refs.tokens.insert(target.clone()) {
                    pending.push(target);
                }
            }
        }
        Ok(refs)
    }

    fn keeps(&self, field: &str, item: &Value) -> bool {
        let id = item["id"].as_str().unwrap_or_default();
        match field {
            "tokens" => self.tokens.contains(id),
            "styles" => self.styles.contains(id),
            "fonts" => self.fonts.contains(id),
            "assets" => self.assets.contains(id),
            _ => true,
        }
    }

    fn walk(&mut self, value: &Value) {
        match value {
            Value::Object(fields) => {
                for (key, child) in fields {
                    let set = match key.as_str() {
                        "tokenId" | "aliasId" => &mut self.tokens,
                        "styleId" => &mut self.styles,
                        "fontId" => &mut self.fonts,
                        "assetId" => &mut self.assets,
                        _ => {
                            self.walk(child);
                            continue;
                        }
                    };
                    match child.as_str() {
                        Some(id) => {
                            set.insert(id.to_owned());
                        }
                        None => self.walk(child),
                    }
                }
            }
            Value::Array(items) => items.iter().for_each(|item| self.walk(item)),
            _ => {}
        }
    }
}

/// `resolved` unchanged without compact; otherwise only the keys that differ from `authored`.
pub(super) fn resolved_props(compact: bool, authored: &Value, resolved: Value) -> Value {
    match resolved {
        Value::Object(fields) if compact => fields
            .into_iter()
            .filter(|(key, value)| authored.get(key) != Some(value))
            .collect::<Map<_, _>>()
            .into(),
        other => other,
    }
}

/// The items of one resource list that this read returns: all of them without compact.
pub(super) fn kept<'a>(
    items: &'a Value,
    field: &'a str,
    compact: Option<&'a Referenced>,
) -> impl Iterator<Item = &'a Value> {
    items
        .as_array()
        .into_iter()
        .flatten()
        .filter(move |item| compact.is_none_or(|refs| refs.keeps(field, item)))
}

/// Declares the compact transforms on the response; a no-op for the default read.
pub(super) fn mark(response: &mut Value, compact: Option<&Referenced>) {
    if compact.is_some() {
        response["resolvedPropsMode"] = json!("differing");
        response["resources"] = json!("referenced");
    }
}
