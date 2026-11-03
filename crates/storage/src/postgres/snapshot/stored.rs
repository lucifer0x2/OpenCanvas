//! Entity rows exactly as stored, kept by a write transaction so it can find
//! the rows its candidate changes without re-serializing the base revision.
use super::*;
use crate::validate::Candidate;
use serde::Serialize;
use std::collections::BTreeMap;
use transaction::Written;

/// (entity id, stored JSON text), sorted by id like the candidate's maps.
type Rows = Vec<(String, String)>;

/// The stored text of every entity in one write transaction's base revision.
#[derive(Default)]
pub(in crate::postgres) struct Stored {
    pub(super) nodes: Rows,
    pub(super) tokens: Rows,
    pub(super) styles: Rows,
    pub(super) assets: Rows,
    pub(super) fonts: Rows,
    pub(super) notes: Rows,
}

impl Stored {
    pub(super) fn sort(&mut self) {
        for rows in [
            &mut self.nodes,
            &mut self.tokens,
            &mut self.styles,
            &mut self.assets,
            &mut self.fonts,
            &mut self.notes,
        ] {
            rows.sort_by(|a, b| a.0.cmp(&b.0));
        }
    }

    /// The rows to upsert and delete so storage holds `after`.
    ///
    /// Each candidate entity is serialized once and compared with the text the
    /// base row was loaded from. Domain equality treats -0.0 and +0.0 as equal,
    /// so it is never used: persistence keeps the exact JSON, nested numbers
    /// included. Stored text that a re-serialization would not reproduce costs
    /// one redundant upsert of the new text, never a missed change.
    pub fn delta(&self, after: &Candidate) -> Result<Written> {
        let mut rows = Written {
            kinds: vec![],
            ids: vec![],
            payloads: vec![],
            removed_kinds: vec![],
            removed_ids: vec![],
        };
        merge("node", &self.nodes, &after.nodes, &mut rows)?;
        merge("token", &self.tokens, &after.tokens, &mut rows)?;
        merge("style", &self.styles, &after.styles, &mut rows)?;
        merge("asset", &self.assets, &after.assets, &mut rows)?;
        merge("font", &self.fonts, &after.fonts, &mut rows)?;
        merge("note", &self.notes, &after.notes, &mut rows)?;
        Ok(rows)
    }
}

/// One ordered pass over the base rows and the candidate map of one kind.
fn merge<T: Serialize>(
    kind: &str,
    stored: &Rows,
    after: &BTreeMap<String, T>,
    rows: &mut Written,
) -> Result<()> {
    let mut base = stored.iter().peekable();
    for (id, value) in after {
        while let Some((gone, _)) = base.next_if(|(old, _)| old < id) {
            rows.removed_kinds.push(kind.into());
            rows.removed_ids.push(gone.clone());
        }
        let payload = serde_json::to_string(value)?;
        // Consume every base row with this id, so none is mistaken for a removal.
        let mut unchanged = false;
        while let Some((_, text)) = base.next_if(|(old, _)| old == id) {
            unchanged |= *text == payload;
        }
        if !unchanged {
            rows.kinds.push(kind.into());
            rows.ids.push(id.clone());
            rows.payloads.push(payload);
        }
    }
    for (gone, _) in base {
        rows.removed_kinds.push(kind.into());
        rows.removed_ids.push(gone.clone());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
