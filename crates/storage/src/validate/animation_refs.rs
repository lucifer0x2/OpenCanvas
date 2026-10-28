//! Token and style references inside `props.animations`: timing takes number tokens,
//! keyframe values take number tokens and `fill` a color token or fill style.
use super::Candidate;
use crate::error::*;
use serde_json::Value;

const TIMING: [&str; 2] = ["duration", "delay"];
const NUMBERS: [&str; 5] = ["opacity", "translateX", "translateY", "scale", "rotate"];

impl Candidate {
    /// Resolves references in place at the current theme. Malformed shapes are left for
    /// `check_animations`, which reports them with their full path.
    pub(super) fn resolve_animations(&self, animations: &mut Value) -> Result<()> {
        let Some(items) = animations.as_array_mut() else {
            return Ok(());
        };
        for (i, animation) in items.iter_mut().enumerate() {
            let at = format!("props.animations[{i}]");
            let Some(animation) = animation.as_object_mut() else {
                continue;
            };
            for key in TIMING {
                if let Some(v) = animation.get_mut(key) {
                    self.resolve_at(v, "number", &format!("{at}.{key}"))?;
                }
            }
            let Some(frames) = animation.get_mut("keyframes").and_then(Value::as_array_mut) else {
                continue;
            };
            for (j, frame) in frames.iter_mut().enumerate() {
                let Some(frame) = frame.as_object_mut() else {
                    continue;
                };
                for (key, v) in frame.iter_mut() {
                    let expected = match key.as_str() {
                        "fill" => "color",
                        k if NUMBERS.contains(&k) => "number",
                        _ => continue,
                    };
                    self.resolve_at(v, expected, &format!("{at}.keyframes[{j}].{key}"))?;
                }
            }
        }
        Ok(())
    }

    /// A reference error names the animation field, not the bare `tokenId`.
    fn resolve_at(&self, v: &mut Value, expected: &str, field: &str) -> Result<()> {
        if v.is_object() {
            *v = self
                .resolve_value(v, expected)
                .map_err(|e| StorageError::validation(field, &e.message))?;
        }
        Ok(())
    }
}
