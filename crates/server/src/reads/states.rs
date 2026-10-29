//! Context variants contain only keys that differ from the resting resolved value.
use serde_json::{Map, Value};

pub(super) fn requested(req: &super::ReadRequest) -> bool {
    req.states_mode.as_deref() == Some("resolved")
        && req
            .properties
            .as_ref()
            .is_none_or(|f| f.iter().any(|k| k == "states"))
}

pub(super) fn differing(props: &Value, kind: &opencanvas_storage::NodeKind) -> Option<Value> {
    let states = props.get("states")?.as_object()?;
    Some(Value::Object(
        states
            .iter()
            .map(|(name, state)| {
                let overrides = state
                    .as_object()
                    .unwrap()
                    .iter()
                    .filter(|(key, value)| {
                        let default = match key.as_str() {
                            "opacity" => Value::from(1),
                            "radius" => Value::from(0),
                            "fill" if *kind == opencanvas_storage::NodeKind::Text => {
                                Value::from("#182230")
                            }
                            _ => Value::Null,
                        };
                        let resting = props.get(*key).unwrap_or(&default);
                        if resting.is_number() && value.is_number() {
                            resting.as_f64() != value.as_f64()
                        } else {
                            resting != *value
                        }
                    })
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<Map<_, _>>();
                (name.clone(), Value::Object(overrides))
            })
            .collect(),
    ))
}
