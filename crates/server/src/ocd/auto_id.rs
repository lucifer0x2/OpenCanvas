//! IDs the compiler writes for nodes without `#id` and notes without `id=`. A node's ID is
//! `<anchor>-<kind>-<hash>`: the anchor is the nearest ancestor written with `#id` (or the
//! `@in` parent), so IDs stay short at any depth; the hash covers the full parent ID, the
//! kind, the node's text, name or component, and how many same-labelled siblings precede
//! it. The same source gives the same IDs, a sibling added elsewhere moves none of them, and
//! a node in another batch or under another parent gets a different one, so a later batch
//! cannot address it by guessing a position. `apply_batch` reports them as `autoIds`.
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Hex digits of the node hash: 32 bits, far from colliding within one parent's anchor.
const HASH_HEX: usize = 8;
const MAX_ID: usize = 128;

/// FNV-1a 64 over `parts` separated by NUL: stable across builds and platforms, unlike
/// `DefaultHasher`.
pub fn hash(parts: &[&str]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for (i, part) in parts.iter().enumerate() {
        let separator = (i > 0).then_some(0u8);
        for byte in separator.into_iter().chain(part.bytes()) {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

/// `prefix` cut to `room` bytes. Callers pass IDs that passed `value::id`, which are ASCII,
/// so any byte index is a char boundary.
pub fn cut(prefix: &str, room: usize) -> &str {
    &prefix[..prefix.len().min(room)]
}

/// Counts same-labelled siblings under one parent while its children are compiled.
#[derive(Default)]
pub struct Siblings(HashMap<(String, String), usize>);

impl Siblings {
    /// The ID of the next child of `parent` (anchored at `anchor`) of source `kind` with
    /// these props. `taken` reports IDs already used in the batch; a hash collision moves
    /// to the next salt instead of failing.
    pub fn next(
        &mut self,
        anchor: &str,
        parent: &str,
        kind: &str,
        props: &Map<String, Value>,
        taken: impl Fn(&str) -> bool,
    ) -> String {
        let label = ["text", "name", "componentId"]
            .iter()
            .find_map(|key| props.get(*key).and_then(Value::as_str))
            .unwrap_or_default()
            .to_owned();
        let seen = self.0.entry((kind.to_owned(), label.clone())).or_default();
        *seen += 1;
        let anchor = cut(anchor, MAX_ID - kind.len() - HASH_HEX - 2);
        let mut salt = *seen;
        loop {
            let count = salt.to_string();
            let digest = hash(&[parent, kind, &label, &count]) >> (64 - 4 * HASH_HEX);
            let id = format!("{anchor}-{kind}-{digest:0width$x}", width = HASH_HEX);
            if !taken(&id) {
                return id;
            }
            salt += 1_000_000;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn props(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn ids_depend_on_label_and_occurrence_not_position() {
        let mut a = Siblings::default();
        let hi = props(json!({"text":"Hi"}));
        let first = a.next("card", "card", "text", &hi, |_| false);
        let second = a.next("card", "card", "text", &hi, |_| false);
        assert!(first.starts_with("card-text-") && first.len() == "card-text-".len() + 8);
        assert_ne!(first, second);
        // Another sibling first does not move the "Hi" texts.
        let mut b = Siblings::default();
        b.next("card", "card", "rect", &Map::new(), |_| false);
        assert_eq!(b.next("card", "card", "text", &hi, |_| false), first);
        // A taken ID moves to another salt.
        let mut c = Siblings::default();
        let moved = c.next("card", "card", "text", &hi, |id| id == first);
        assert!(moved != first && moved.starts_with("card-text-"));
    }

    #[test]
    fn long_anchors_are_cut_to_fit_the_id_limit() {
        let anchor = "a".repeat(200);
        let id = Siblings::default().next(&anchor, "p", "image", &Map::new(), |_| false);
        assert_eq!(id.len(), MAX_ID);
    }
}
