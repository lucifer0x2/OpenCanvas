//! Keyframe checks of one `props.animations` entry: keys, values, WAAPI offsets and the
//! paired translate axes. Values arrive with tokens and styles already resolved.
use super::{easing::check_easing, values::color};
use crate::{
    animation_catalog::{keyframe_range, AnimationKey, ANIMATED_PROPS, KEYFRAME_KEYS},
    error::*,
    props_suggest::suggest_from,
};
use serde_json::{Map, Value};

/// Checks every keyframe at `at` (`props.animations[i]`) and returns, per animated key,
/// the index of the first keyframe that sets it.
pub(super) fn check_keyframes(
    frames: &[Value],
    at: &str,
    hover: bool,
) -> Result<Vec<(&'static str, usize)>> {
    let mut offsets = Vec::with_capacity(frames.len());
    let mut first: Vec<(&'static str, usize)> = Vec::new();
    for (j, frame) in frames.iter().enumerate() {
        let here = format!("{at}.keyframes[{j}]");
        let frame = frame
            .as_object()
            .ok_or_else(|| StorageError::validation(&here, "Each keyframe must be an object"))?;
        reject_unknown(frame, KEYFRAME_KEYS, &here, |_, _| None)?;
        let offset = frame.get("offset");
        if let Some(v) = offset {
            in_range(
                v,
                0.,
                1.,
                &format!("{here}.offset"),
                "offset is a fraction 0..1",
            )?;
            if hover && v.as_f64() != Some(1.) {
                return Err(StorageError::validation(
                    &format!("{here}.offset"),
                    "A hover animation goes from the authored props to one keyframe: omit offset or use 1",
                ));
            }
        }
        offsets.push(offset.and_then(Value::as_f64));
        if let Some(v) = frame.get("easing") {
            check_easing(v, &format!("{here}.easing"))?;
        }
        let mut animated = false;
        for key in ANIMATED_PROPS {
            let Some(v) = frame.get(key) else { continue };
            animated = true;
            let field = format!("{here}.{key}");
            match keyframe_range(key) {
                Some((min, max)) => in_range(v, min, max, &field, "value is out of range")?,
                None => color(v).map_err(|e| StorageError::validation(&field, &e.message))?,
            }
            if !first.iter().any(|(k, _)| *k == key) {
                first.push((key, j));
            }
        }
        if !animated {
            return Err(StorageError::validation(
                &here,
                "A keyframe needs at least one of opacity, translateX, translateY, scale, rotate, fill",
            ));
        }
    }
    check_offsets(&offsets, at)?;
    check_translate_pairs(frames, at)?;
    Ok(first)
}

/// WAAPI computed offsets (lone keyframe = 1, first = 0, last = 1, gaps spread evenly)
/// must increase strictly.
fn check_offsets(authored: &[Option<f64>], at: &str) -> Result<()> {
    let n = authored.len();
    let mut offsets = authored.to_vec();
    if n == 1 {
        offsets[0].get_or_insert(1.);
    } else {
        offsets[0].get_or_insert(0.);
        offsets[n - 1].get_or_insert(1.);
    }
    let mut start = 0;
    for k in 1..n {
        if let Some(end) = offsets[k] {
            let from = offsets[start].unwrap();
            let span = (k - start) as f64;
            for (step, offset) in offsets[start + 1..k].iter_mut().enumerate() {
                *offset = Some(from + (end - from) * (step + 1) as f64 / span);
            }
            start = k;
        }
    }
    for j in 1..n {
        let (previous, current) = (offsets[j - 1].unwrap(), offsets[j].unwrap());
        if current <= previous {
            return Err(StorageError::validation(
                &format!("{at}.keyframes[{j}].offset"),
                &format!(
                    "Offsets must increase: keyframes[{}] is at {previous}",
                    j - 1
                ),
            ));
        }
    }
    Ok(())
}

/// translateX and translateY are one CSS `translate`: when an animation sets both, every
/// keyframe sets both or neither.
fn check_translate_pairs(frames: &[Value], at: &str) -> Result<()> {
    let has = |key| frames.iter().any(|f| f.get(key).is_some());
    if !(has("translateX") && has("translateY")) {
        return Ok(());
    }
    for (j, frame) in frames.iter().enumerate() {
        let (x, y) = (frame.get("translateX"), frame.get("translateY"));
        let (set, missing) = match (x, y) {
            (Some(_), None) => ("translateX", "translateY"),
            (None, Some(_)) => ("translateY", "translateX"),
            _ => continue,
        };
        return Err(StorageError::validation(
            &format!("{at}.keyframes[{j}].{missing}"),
            &format!(
                "translateX and translateY form one CSS translate: keyframes[{j}] sets {set}, so set {missing} there too, or neither"
            ),
        ));
    }
    Ok(())
}

/// Rejects the first key outside `table`, naming the table's keys and the likely intent.
/// `special` may suggest a key from the rejected key and its value.
pub(super) fn reject_unknown(
    object: &Map<String, Value>,
    table: &[AnimationKey],
    at: &str,
    special: impl Fn(&str, &Value) -> Option<&'static str>,
) -> Result<()> {
    let Some((key, value)) = object
        .iter()
        .find(|(k, _)| !table.iter().any(|spec| spec.key == k.as_str()))
    else {
        return Ok(());
    };
    let suggestion = special(key, value)
        .or_else(|| suggest_from(key, table.iter().map(|spec| (spec.key, spec.aliases))));
    let hint = suggestion
        .map(|s| format!("; did you mean '{s}'?"))
        .unwrap_or_default();
    let mut e = StorageError::validation(
        &format!("{at}.{key}"),
        &format!("Unsupported key '{key}' at {at}{hint}"),
    );
    let node = e.node_context();
    node.allowed_keys = Some(table.iter().map(|spec| spec.key.into()).collect());
    node.suggestion = suggestion.map(Into::into);
    Err(e)
}

pub(super) fn in_range(v: &Value, min: f64, max: f64, field: &str, message: &str) -> Result<()> {
    match v.as_f64() {
        Some(n) if n.is_finite() && (min..=max).contains(&n) => Ok(()),
        _ => Err(StorageError::validation(
            field,
            &format!("{message}: expected a number {min}..{max}"),
        )),
    }
}
