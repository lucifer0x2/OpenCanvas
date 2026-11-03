//! Paint patch semantics shared by expansion and full merged-value validation.
use serde_json::Value;

pub(crate) fn merge_props(base: &mut Value, patch: &serde_json::Map<String, Value>) {
    for (key, value) in patch {
        match key.as_str() {
            "stroke" => merge_object(&mut base[key], value),
            "states" => {
                if let Some(states) = value.as_object() {
                    if !base[key].is_object() {
                        base[key] = serde_json::json!({});
                    }
                    for (state, props) in states {
                        if let Some(props) = props.as_object() {
                            if !base[key][state].is_object() {
                                base[key][state] = serde_json::json!({});
                            }
                            merge_props(&mut base[key][state], props);
                        } else {
                            base[key][state] = props.clone();
                        }
                    }
                } else {
                    base[key] = value.clone();
                }
            }
            _ => base[key] = value.clone(),
        }
    }
}

fn merge_object(base: &mut Value, patch: &Value) {
    if let (Some(base), Some(patch)) = (base.as_object_mut(), patch.as_object()) {
        base.extend(patch.clone());
    } else {
        *base = patch.clone();
    }
}
