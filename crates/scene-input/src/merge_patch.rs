//! `update_node` with `merge:true`: an RFC 7386 JSON merge patch of the node's props. Objects
//! merge key by key, `null` deletes a key, and any other value (arrays included) replaces.
//! A patch object holding `tokenId`, `styleId` or `type` is a value (a reference or a
//! gradient), not a record, so it replaces the stored value instead of mixing with it.
use serde_json::{Map, Value};

/// Keys that make an object one atomic value.
const VALUE_KEYS: [&str; 3] = ["tokenId", "styleId", "type"];

/// Merges `patch` into the stored top-level props.
pub fn merge_props(stored: &mut Map<String, Value>, patch: &Map<String, Value>) {
    for (key, value) in patch {
        if value.is_null() {
            stored.remove(key);
        } else {
            merge(stored.entry(key.clone()).or_insert(Value::Null), value);
        }
    }
}

/// True for a record that a merge patch merges into: an object that is not a value.
pub fn merges(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|fields| !VALUE_KEYS.iter().any(|key| fields.contains_key(*key)))
}

fn merge(target: &mut Value, patch: &Value) {
    match patch.as_object() {
        Some(fields) if merges(patch) => {
            if !target.is_object() {
                *target = Value::Object(Map::new());
            }
            merge_props(target.as_object_mut().expect("set above"), fields);
        }
        _ => *target = patch.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn merged(stored: Value, patch: Value) -> Value {
        let mut stored = stored.as_object().unwrap().clone();
        merge_props(&mut stored, patch.as_object().unwrap());
        Value::Object(stored)
    }

    #[test]
    fn records_merge_nulls_delete_and_values_replace() {
        let stored = json!({"name":"Card","layout":{"mode":"column","gap":4,"padding":[8,8]},
            "overrides":{"ph":{"text":"x","fill":{"styleId":"s"}},"title":{"text":"T"}},
            "fill":{"type":"linear","angle":90,"stops":[]}});
        let patch = json!({"layout":{"gap":8,"padding":[4]},"name":null,
            "overrides":{"ph":{"fill":{"tokenId":"pri"}}},"fill":"#fff"});
        assert_eq!(
            merged(stored, patch),
            json!({"layout":{"mode":"column","gap":8,"padding":[4]},
                "overrides":{"ph":{"text":"x","fill":{"tokenId":"pri"}},"title":{"text":"T"}},
                "fill":"#fff"})
        );
    }

    #[test]
    fn a_missing_or_scalar_target_becomes_the_patched_record() {
        let patch = json!({"stroke":{"color":"#000","dash":null},"layout":{"gap":2}});
        assert_eq!(
            merged(json!({"stroke":"none"}), patch),
            json!({"stroke":{"color":"#000"},"layout":{"gap":2}})
        );
    }
}
