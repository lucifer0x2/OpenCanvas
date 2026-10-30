//! A fill keyframe animates a solid color under the resting paint (CSS `background-color`
//! beneath `background-image`), so a gradient resting fill would hide it. Checked on resolved
//! props, where an authored gradient and one from a fill style (`styleId`) look the same.
//! Color tokens and themes are solid-only, so they never turn a resting fill into a gradient.
use crate::error::*;
use serde_json::{Map, Value};

pub(super) fn check_animated_fill(props: &Map<String, Value>) -> Result<()> {
    let Some(gradient) = props
        .get("fill")
        .and_then(|fill| fill.get("type"))
        .and_then(Value::as_str)
    else {
        return Ok(());
    };
    let Some(animations) = props.get("animations").and_then(Value::as_array) else {
        return Ok(());
    };
    for (i, animation) in animations.iter().enumerate() {
        let frames = animation.get("keyframes").and_then(Value::as_array);
        if let Some(j) = frames.and_then(|f| f.iter().position(|k| k.get("fill").is_some())) {
            return Err(StorageError::validation(
                &format!("props.animations[{i}].keyframes[{j}].fill"),
                &format!(
                    "Fill animation requires a solid resting fill; this node's fill is a {gradient} gradient, which would hide the animated color. Make the resting fill solid, or animate opacity/transform instead"
                ),
            ));
        }
    }
    Ok(())
}
