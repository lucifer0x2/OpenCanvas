//! Navigation authored as data: `props.links` on a source node says what a click or hover
//! does, and `props.flowStart` names a root frame as the first screen of a flow. One reading
//! with defaults filled, shared by the validator and every read, so defaults cannot drift.
use serde::Serialize;
use serde_json::Value;

/// Accepted `on` values; the first is the default.
pub const LINK_TRIGGERS: [&str; 2] = ["click", "hover"];
/// Accepted `action` values; the first is the default.
pub const LINK_ACTIONS: [&str; 4] = ["navigate", "overlay", "back", "close"];
/// Keys of one link object.
pub const LINK_KEYS: [&str; 3] = ["to", "on", "action"];
/// Longest flow name in bytes.
pub const MAX_FLOW_NAME: usize = 64;

/// One authored link with its defaults filled: `on` click, `action` navigate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Link {
    pub on: String,
    pub action: String,
    /// The destination root frame ID; absent for back and close.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

impl Link {
    /// Whether this action names a destination frame (navigate, overlay).
    pub fn needs_target(action: &str) -> bool {
        matches!(action, "navigate" | "overlay")
    }
}

/// The links of one node's authored props, defaults filled. Entries that are not objects are
/// skipped: committed snapshots only hold validated links.
pub fn node_links(props: &Value) -> Vec<Link> {
    let Some(items) = props.get("links").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(Value::as_object)
        .map(|link| Link {
            on: text(link.get("on"), LINK_TRIGGERS[0]),
            action: text(link.get("action"), LINK_ACTIONS[0]),
            to: link.get("to").and_then(Value::as_str).map(Into::into),
        })
        .collect()
}

/// The flow a root frame starts, if any.
pub fn flow_start(props: &Value) -> Option<&str> {
    props.get("flowStart").and_then(Value::as_str)
}

fn text(value: Option<&Value>, default: &str) -> String {
    value.and_then(Value::as_str).unwrap_or(default).into()
}
