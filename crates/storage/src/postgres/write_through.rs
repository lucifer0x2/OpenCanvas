//! Post-commit cache fill. A committed write already holds every row of the
//! new revision, so readers after an in-process commit skip the SQL reload.
use super::*;
use crate::{Snapshot, SnapshotCache};
use snapshot::Binaries;
use std::{collections::HashSet, sync::Arc};

/// The rows of one write transaction, enough to rebuild the committed state.
/// Built inside the transaction; assembled and cached only after its commit.
pub(super) struct Committed {
    /// Entity rows as stored before the write, without readiness applied.
    before: Snapshot,
    /// Binary hashes of the project at commit, including this write's import.
    binaries: Binaries,
    written: transaction::Written,
    revision: u64,
    generation: i64,
    namespace: String,
}

impl Committed {
    pub fn new(
        before: Snapshot,
        binaries: Binaries,
        written: transaction::Written,
        revision: u64,
        project: (i64, String),
    ) -> Self {
        let (generation, namespace) = project;
        Self {
            before,
            binaries,
            written,
            revision,
            generation,
            namespace,
        }
    }

    /// Call only after the transaction committed. Fills a cache bound to the
    /// namespace that the write transaction observed; `put` keeps newer entries.
    pub fn fill(self, cache: Option<&SnapshotCache>) {
        let Some(cache) = cache.filter(|cache| cache.namespace() == self.namespace) else {
            return;
        };
        let generation = self.generation;
        // Rows were serialized by this write; a parse failure would skip only
        // the fill, never fail the committed request.
        if let Ok(snapshot) = self.snapshot() {
            cache.put(&Arc::new(snapshot), generation);
        }
    }

    /// The committed revision exactly as an uncached `snapshot::read` returns
    /// it: the stored rows minus deletions, plus upserts parsed from the text
    /// that was stored, then the shared ordering and readiness.
    fn snapshot(self) -> Result<Snapshot> {
        let Self {
            mut before,
            binaries,
            written,
            revision,
            ..
        } = self;
        let touched: HashSet<(&str, &str)> = written
            .kinds
            .iter()
            .zip(&written.ids)
            .chain(written.removed_kinds.iter().zip(&written.removed_ids))
            .map(|(kind, id)| (kind.as_str(), id.as_str()))
            .collect();
        macro_rules! retain {
            ($field:ident, $kind:literal) => {
                if touched.iter().any(|(kind, _)| *kind == $kind) {
                    before
                        .$field
                        .retain(|row| !touched.contains(&($kind, row.id.as_str())));
                }
            };
        }
        retain!(nodes, "node");
        retain!(tokens, "token");
        retain!(styles, "style");
        retain!(assets, "asset");
        retain!(fonts, "font");
        retain!(notes, "note");
        let rows = written.kinds.iter().zip(&written.payloads);
        for (kind, data) in rows {
            match kind.as_str() {
                "node" => before.nodes.push(serde_json::from_str(data)?),
                "token" => before.tokens.push(serde_json::from_str(data)?),
                "style" => before.styles.push(serde_json::from_str(data)?),
                "asset" => before.assets.push(serde_json::from_str(data)?),
                "font" => before.fonts.push(serde_json::from_str(data)?),
                "note" => before.notes.push(serde_json::from_str(data)?),
                _ => unreachable!("persist writes only domain entity kinds"),
            }
        }
        before.revision = revision;
        snapshot::sort(&mut before);
        snapshot::apply_readiness(&mut before, &binaries);
        Ok(before)
    }
}

#[cfg(test)]
mod tests;
