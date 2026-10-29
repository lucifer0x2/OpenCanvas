//! The `## Animations` section of `props.md`, rendered from storage's animation key tables.
use opencanvas_storage::{AnimationKey, ANIMATION_KEYS, KEYFRAME_KEYS};
use std::fmt::Write;

pub(super) fn section() -> String {
    let mut out = String::from(
        "\n## Animations\n\n\
         `animations` declares motion of a node's own visual props; props stay the resting state. \
         Errors name the full path, for example `props.animations[0].keyframes[1].offset`.\n",
    );
    table(&mut out, "Animation keys", ANIMATION_KEYS);
    table(&mut out, "Keyframe keys", KEYFRAME_KEYS);
    out.push_str(
        "\nRules: each property is animated by one animation per node (translateX and translateY \
         count as one translate); when an animation sets both translate axes, every keyframe sets \
         both or neither; `on:\"hover\"` has exactly one keyframe (offset omitted or 1) and no \
         iterations, direction or fillMode; a keyframe sets at least one animated property.\n",
    );
    out
}

fn table(out: &mut String, title: &str, keys: &[AnimationKey]) {
    write!(
        out,
        "\n### {title}\n\n| Key | Value | Not this |\n| --- | --- | --- |\n"
    )
    .unwrap();
    for spec in keys {
        let aliases: Vec<String> = spec.aliases.iter().map(|a| format!("`{a}`")).collect();
        writeln!(
            out,
            "| `{}` | {} | {} |",
            spec.key,
            spec.value.replace('|', "\\|"),
            aliases.join(", ")
        )
        .unwrap();
    }
}
