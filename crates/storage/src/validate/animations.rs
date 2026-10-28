//! Shape checks of `props.animations` (resolved values): animation keys and timing, the
//! hover form, and one animation per animated property of a node.
use super::{
    easing::check_easing,
    keyframes::{check_keyframes, in_range, reject_unknown},
    values::validate_slug,
};
use crate::{
    animation_catalog::{
        channel, ANIMATION_KEYS, DIRECTIONS, FILL_MODES, MAX_ANIMATIONS, MAX_KEYFRAMES, TRIGGERS,
    },
    error::*,
};
use serde_json::{Map, Value};

const AT: &str = "props.animations";

pub(super) fn check_animations(value: &Value) -> Result<()> {
    let items = value.as_array().ok_or_else(|| {
        StorageError::validation(
            AT,
            "animations must be an array of {name, duration, keyframes}",
        )
    })?;
    if items.len() > MAX_ANIMATIONS {
        return Err(StorageError::validation(
            AT,
            &format!("At most {MAX_ANIMATIONS} animations per node"),
        ));
    }
    let mut names: Vec<&str> = Vec::new();
    // Animated property (translate for both axes) → the animation that owns it.
    let mut owners: Vec<(&str, usize, &str)> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let at = format!("{AT}[{i}]");
        let animation = item
            .as_object()
            .ok_or_else(|| StorageError::validation(&at, "Each animation must be an object"))?;
        reject_unknown(animation, ANIMATION_KEYS, &at, fill_mode_hint)?;
        let name = check_name(animation, &at)?;
        if let Some(first) = names.iter().position(|n| *n == name) {
            return Err(StorageError::validation(
                &format!("{at}.name"),
                &format!("Name '{name}' is already used by animations[{first}]"),
            ));
        }
        names.push(name);
        let hover = check_timing(animation, &at)?;
        let frames = keyframes(animation, &at, hover)?;
        for (key, j) in check_keyframes(frames, &at, hover)? {
            let property = channel(key);
            if let Some((_, owner, owner_name)) = owners.iter().find(|(p, _, _)| *p == property) {
                if *owner != i {
                    return Err(StorageError::validation(
                        &format!("{at}.keyframes[{j}].{key}"),
                        &format!(
                            "{property} is already animated by animations[{owner}] '{owner_name}'; merge the keyframes or animate a wrapping group"
                        ),
                    ));
                }
                continue;
            }
            owners.push((property, i, name));
        }
    }
    Ok(())
}

/// `fill` at the animation level with a fill-mode value meant `fillMode`.
fn fill_mode_hint(key: &str, value: &Value) -> Option<&'static str> {
    (key == "fill" && value.as_str().is_some_and(|v| FILL_MODES.contains(&v))).then_some("fillMode")
}

fn check_name<'a>(animation: &'a Map<String, Value>, at: &str) -> Result<&'a str> {
    let field = format!("{at}.name");
    let name = animation
        .get("name")
        .ok_or_else(|| StorageError::validation(&field, "Every animation needs a name"))?;
    let name = name.as_str().unwrap_or("");
    validate_slug(name).map_err(|e| StorageError::validation(&field, &e.message))?;
    Ok(name)
}

/// Checks trigger and timing; returns whether this is a hover animation.
fn check_timing(animation: &Map<String, Value>, at: &str) -> Result<bool> {
    let field = |key: &str| format!("{at}.{key}");
    let on = one_of(animation.get("on"), &TRIGGERS, &field("on"))?;
    let duration = animation.get("duration").ok_or_else(|| {
        StorageError::validation(&field("duration"), "duration is required (milliseconds)")
    })?;
    if duration.as_f64().is_some_and(|ms| ms < 16.) {
        return Err(StorageError::validation(
            &field("duration"),
            "duration is milliseconds (e.g. 400), not seconds; the minimum is 16",
        ));
    }
    in_range(
        duration,
        16.,
        60_000.,
        &field("duration"),
        "duration is milliseconds",
    )?;
    if let Some(delay) = animation.get("delay") {
        in_range(delay, 0., 60_000., &field("delay"), "delay is milliseconds")?;
    }
    if let Some(easing) = animation.get("easing") {
        check_easing(easing, &field("easing"))?;
    }
    if let Some(count) = animation.get("iterations") {
        if count != "infinite" {
            in_range(
                count,
                f64::MIN_POSITIVE,
                1000.,
                &field("iterations"),
                "iterations is a count > 0 or \"infinite\"",
            )?;
        }
    }
    one_of(animation.get("direction"), &DIRECTIONS, &field("direction"))?;
    one_of(animation.get("fillMode"), &FILL_MODES, &field("fillMode"))?;
    let hover = on == "hover";
    if hover {
        if let Some(key) = ["iterations", "direction", "fillMode"]
            .into_iter()
            .find(|k| animation.contains_key(*k))
        {
            return Err(StorageError::validation(
                &field(key),
                "A hover animation is a transition to one keyframe and back: it takes no iterations, direction or fillMode",
            ));
        }
    }
    Ok(hover)
}

fn keyframes<'a>(animation: &'a Map<String, Value>, at: &str, hover: bool) -> Result<&'a [Value]> {
    let field = format!("{at}.keyframes");
    let frames = animation
        .get("keyframes")
        .and_then(Value::as_array)
        .filter(|f| (1..=MAX_KEYFRAMES).contains(&f.len()))
        .ok_or_else(|| {
            StorageError::validation(
                &field,
                &format!("keyframes must be an array of 1..{MAX_KEYFRAMES} keyframes"),
            )
        })?;
    if hover && frames.len() != 1 {
        return Err(StorageError::validation(
            &field,
            "A hover animation goes from the authored props to one keyframe",
        ));
    }
    Ok(frames)
}

/// A string from `allowed`, or the first (default) when absent.
fn one_of<'a>(value: Option<&'a Value>, allowed: &[&'static str], field: &str) -> Result<&'a str> {
    match value {
        None => Ok(allowed[0]),
        Some(v) => v.as_str().filter(|s| allowed.contains(s)).ok_or_else(|| {
            StorageError::validation(field, &format!("Use one of {}", allowed.join(", ")))
        }),
    }
}
