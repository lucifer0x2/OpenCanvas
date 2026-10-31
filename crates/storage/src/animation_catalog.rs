//! The keys of `props.animations`: CSS/WAAPI-like keyframes of visual properties that never
//! change layout. One table feeds the validator, its "did you mean" suggestions and the
//! published `skill://opencanvas/props.md`.

/// One key of an animation or of a keyframe, with the names agents commonly try instead.
#[derive(Debug, Clone, Copy)]
pub struct AnimationKey {
    pub key: &'static str,
    pub value: &'static str,
    pub aliases: &'static [&'static str],
}

const fn key(
    key: &'static str,
    value: &'static str,
    aliases: &'static [&'static str],
) -> AnimationKey {
    AnimationKey {
        key,
        value,
        aliases,
    }
}

pub const MAX_ANIMATIONS: usize = 8;
pub const MAX_KEYFRAMES: usize = 16;
pub const TRIGGERS: [&str; 3] = ["load", "hover", "click"];
pub const DIRECTIONS: [&str; 4] = ["normal", "reverse", "alternate", "alternate-reverse"];
pub const FILL_MODES: [&str; 4] = ["none", "forwards", "backwards", "both"];

pub const ANIMATION_KEYS: &[AnimationKey] = &[
    key("name", "slug of lowercase letters, digits and internal hyphens, at most 64, unique in the node; required. \"<nodeId>:<name>\" refers to it", &["id", "label", "animationName"]),
    key("on", "\"load\" (default, when the node appears), \"hover\" (transition to the one keyframe while hovered, back on leave) or \"click\" (restart on every click)", &["trigger", "event", "when"]),
    key("duration", "milliseconds 16..60000 (not seconds), or {\"tokenId\"} of a number token; required", &["durationMs", "time", "length"]),
    key("delay", "milliseconds 0..60000 or number token; default 0", &["delayMs", "wait"]),
    key("easing", "linear, ease (default), ease-in, ease-out, ease-in-out, cubic-bezier(x1,y1,x2,y2) with x1,x2 in 0..1, or steps(n[, start|end|jump-start|jump-end]); applies to each segment between keyframes, like CSS", &["ease", "timing", "timingFunction", "curve"]),
    key("iterations", "number > 0 up to 1000, or \"infinite\"; default 1", &["repeat", "loop", "iterationCount", "count"]),
    key("direction", "normal (default), reverse, alternate or alternate-reverse", &["playDirection"]),
    key("fillMode", "none, forwards, backwards or both (default, unlike CSS none)", &["fill-mode", "animationFillMode"]),
    key("keyframes", "array of 1..16 keyframes", &["frames", "steps", "keys", "track", "tracks"]),
];

pub const KEYFRAME_KEYS: &[AnimationKey] = &[
    key("offset", "0..1; omitted offsets spread like WAAPI (first 0, last 1, between evenly); a lone keyframe is offset 1; offsets must increase", &["t", "at", "time", "percent", "progress", "position"]),
    key("easing", "easing of the segment that starts at this keyframe", &["ease", "timing", "timingFunction"]),
    key("opacity", "number 0..1 or number token; replaces the authored opacity", &["alpha"]),
    key("translateX", "pixels -10000..10000 or number token; offset from the laid-out position (CSS translate)", &["x", "left", "dx", "offsetX"]),
    key("translateY", "pixels -10000..10000 or number token; offset from the laid-out position (CSS translate)", &["y", "top", "dy", "offsetY"]),
    key("scale", "number 0..100 or number token; uniform, around the bounds center", &["scaleX", "scaleY", "zoom", "size"]),
    key("rotate", "degrees -36000..36000 or number token; around the bounds center", &["rotation", "angle"]),
    key("fill", "like the fill prop: hex, color token {\"tokenId\"} or solid fill style {\"styleId\"}; the node's resting fill must be solid, not a gradient", &["color", "backgroundColor", "background", "textColor", "bg"]),
];

/// Properties a keyframe may animate. translateX and translateY form one CSS `translate`.
pub const ANIMATED_PROPS: [&str; 6] = [
    "opacity",
    "translateX",
    "translateY",
    "scale",
    "rotate",
    "fill",
];

/// Literal range of a numeric keyframe property.
pub fn keyframe_range(key: &str) -> Option<(f64, f64)> {
    match key {
        "opacity" => Some((0., 1.)),
        "translateX" | "translateY" => Some((-10_000., 10_000.)),
        "scale" => Some((0., 100.)),
        "rotate" => Some((-36_000., 36_000.)),
        _ => None,
    }
}

/// The property an animated key occupies on its node: both translate axes share one.
pub fn channel(key: &str) -> &str {
    match key {
        "translateX" | "translateY" => "translate",
        other => other,
    }
}
