//! Resolution after the caller's explicit full-snapshot validation boundary.
use super::{scope::Resolver, tokens::unsupported_theme, Candidate};
use crate::{Node, Result, Snapshot};
use serde_json::{json, Value};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

thread_local! {
    static BUILDS: Cell<usize> = const { Cell::new(0) };
}

/// Resource builds on this thread; lets tests bound per-request resource work.
pub fn resource_builds() -> usize {
    BUILDS.with(Cell::get)
}

/// Immutable resource maps and declared themes, built once per scoped read.
/// Node resolution borrows them; per-call work is the selected node closure.
pub(crate) struct ScopedResources {
    candidate: Candidate,
    themes: BTreeSet<String>,
}

impl ScopedResources {
    pub(crate) fn new(snapshot: &Snapshot) -> Result<Self> {
        BUILDS.with(|builds| builds.set(builds.get() + 1));
        crate::bench_phase::count("scoped.resource_builds", 1);
        let candidate = Candidate {
            theme: "default".into(),
            nodes: BTreeMap::new(),
            tokens: by_id(&snapshot.tokens, |t| &t.id),
            styles: by_id(&snapshot.styles, |s| &s.id),
            assets: by_id(&snapshot.assets, |a| &a.id),
            fonts: by_id(&snapshot.fonts, |f| &f.id),
            notes: BTreeMap::new(),
            deleted: Default::default(),
        };
        let themes = candidate.validate_tokens()?;
        Ok(Self { candidate, themes })
    }

    /// The requested-theme check of `Candidate::select_theme`, without revalidating.
    pub(crate) fn check_theme(&self, theme: &str) -> Result<()> {
        if self.themes.contains(theme) {
            Ok(())
        } else {
            Err(unsupported_theme())
        }
    }

    pub(crate) fn resolve(&self, nodes: &[Node], theme: &str) -> Result<Vec<Node>> {
        self.check_theme(theme)?;
        let map: BTreeMap<_, _> = nodes.iter().map(|n| (n.id.clone(), n.clone())).collect();
        let resolver = Resolver {
            candidate: &self.candidate,
            theme,
            nodes: &map,
        };
        nodes
            .iter()
            .cloned()
            .map(|mut node| {
                let effective = resolver.effective_theme(&node)?.to_owned();
                node.props = resolver.resolve_scoped_props(&node)?;
                node.props["effectiveTheme"] = Value::String(effective);
                if node.props["visible"] == false {
                    node.props["opacity"] = json!(0);
                }
                Ok(node)
            })
            .collect()
    }

    pub(crate) fn tokens(&self, theme: &str) -> Result<BTreeMap<String, Value>> {
        self.check_theme(theme)?;
        self.candidate
            .tokens
            .keys()
            .map(|id| Ok((id.clone(), self.candidate.token_value(id, theme)?)))
            .collect()
    }
}

fn by_id<T: Clone>(items: &[T], id: impl Fn(&T) -> &String) -> BTreeMap<String, T> {
    items.iter().map(|i| (id(i).clone(), i.clone())).collect()
}
