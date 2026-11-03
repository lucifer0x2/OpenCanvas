//! `animated` on `get_design_context`: frame-wide discovery in preorder then array
//! order. Keep repeated metadata small enough for an ordinary page entry to fit.
use super::pagination::json_len;
use crate::error::{ApiError, Result};
use opencanvas_storage::Node;
use serde_json::{json, Value};

const SUMMARY_BYTES: usize = ",\"animated\":[]".len();
const TRUNCATED_BYTES: usize = ",\"animatedTruncated\":true".len();

pub(super) fn include(response: &mut Value, scope: &[(&Node, usize)], budget: usize) -> Result<()> {
    let entries = scope
        .iter()
        .filter_map(|(node, _)| Some((node, node.props.get("animations")?.as_array()?)))
        .flat_map(|(node, animations)| animations.iter().map(move |a| (node, a)));
    let mut items = Vec::new();
    let mut bytes = SUMMARY_BYTES;
    for (node, animation) in entries {
        let on = animation
            .get("on")
            .and_then(Value::as_str)
            .unwrap_or("load");
        let name = animation["name"].as_str().unwrap_or_default();
        let mut item = json!({"ref": format!("{}:{name}", node.id), "on": on});
        if let Some(source) = animation.get("triggerNodeId").and_then(Value::as_str) {
            item["triggerNodeId"] = json!(source);
        }
        let size = json_len(&item)? + usize::from(!items.is_empty());
        if bytes + size > budget.saturating_sub(TRUNCATED_BYTES) {
            if budget < TRUNCATED_BYTES {
                return Err(ApiError::new(
                    "CONTEXT_ITEM_TOO_LARGE",
                    "Animation discovery metadata cannot fit in the response budget; use a property filter or larger budget",
                ));
            }
            response["animatedTruncated"] = json!(true);
            break;
        }
        bytes += size;
        items.push(item);
    }
    if !items.is_empty() {
        response["animated"] = json!(items);
    }
    Ok(())
}

pub(super) fn has_tracks(scope: &[(&Node, usize)]) -> bool {
    scope.iter().any(|(node, _)| {
        node.props["animations"]
            .as_array()
            .is_some_and(|animations| !animations.is_empty())
    })
}
