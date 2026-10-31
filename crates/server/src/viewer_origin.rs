//! Project only server-generated HTTP links; authored strings and cached jobs stay untouched.
use axum::Extension;
use serde_json::Value;

#[derive(Clone)]
pub(crate) struct ViewerOrigin(pub String);
pub(crate) type Origin = Option<Extension<ViewerOrigin>>;

fn link(value: &mut Value, local: &str, origin: &ViewerOrigin) {
    if let Some(path) = value.as_str().and_then(|s| s.strip_prefix(local)) {
        if path.starts_with("/api/v1/") {
            *value = Value::String(format!("{}{path}", origin.0));
        }
    }
}

pub(crate) fn job(mut value: Value, local: &str, origin: Origin) -> Value {
    let Some(Extension(origin)) = origin else {
        return value;
    };
    for pointer in ["/imageUrl", "/image/uri"] {
        if let Some(url) = value.pointer_mut(pointer) {
            link(url, local, &origin);
        }
    }
    for pointer in ["/scene/fonts", "/scene/assets", "/files"] {
        if let Some(items) = value.pointer_mut(pointer).and_then(Value::as_array_mut) {
            for item in items {
                if let Some(url) = item.get_mut("url") {
                    link(url, local, &origin);
                }
            }
        }
    }
    value
}

pub(crate) fn exports(mut value: Value, local: &str, origin: Origin) -> Value {
    if let Some(items) = value.get_mut("exports").and_then(Value::as_array_mut) {
        for item in items {
            *item = job(item.take(), local, origin.clone());
        }
    }
    value
}
