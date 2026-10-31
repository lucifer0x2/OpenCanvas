//! `skill://opencanvas/props.md`, rendered from the storage validator's property table so
//! the published keys are exactly the keys a commit accepts.
mod animations;
mod instances;

use opencanvas_storage::{allowed_props, NODE_KINDS, PROPS};
use std::{fmt::Write, sync::OnceLock};

pub(crate) const URI: &str = "skill://opencanvas/props.md";
pub(crate) const NAME: &str = "OpenCanvas node properties by kind";

pub(crate) fn text() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(render)
}

/// The doc line listing a kind's keys; tests compare it with validation errors.
pub(crate) fn kind_line(kind: &str, keys: &[&str]) -> String {
    let keys: Vec<String> = keys.iter().map(|key| format!("`{key}`")).collect();
    format!("- `{kind}`: {}", keys.join(", "))
}

fn render() -> String {
    let mut out = String::from(
        "# OpenCanvas node properties\n\n\
         Generated from the validator's property table: these are the only keys `create_node` and \
         `update_node` accept in `props`. Any other key, or a key on the wrong kind, fails with \
         `VALIDATION_FAILED`, `field: \"props.<key>\"`, `nodeId`, `nodeKind`, `allowedKeys` for that \
         kind and, when one is close, `suggestion`. `update_node` merges its keys into the node's props.\n\n\
         ## Keys by kind\n\n",
    );
    for kind in &NODE_KINDS {
        writeln!(out, "{}", kind_line(kind.as_str(), &allowed_props(kind))).unwrap();
    }
    out.push_str(
        "\n## Keys\n\n\
         | Key | Kinds | Required | Value | Not this |\n\
         | --- | --- | --- | --- | --- |\n",
    );
    for spec in PROPS {
        let aliases: Vec<String> = spec.aliases.iter().map(|a| format!("`{a}`")).collect();
        writeln!(
            out,
            "| `{}` | {} | {} | {} | {} |",
            spec.key,
            spec.scope.label(),
            if spec.required { "yes" } else { "no" },
            spec.value.replace('|', "\\|"),
            aliases.join(", ")
        )
        .unwrap();
    }
    out.push_str(
        "\n## Notes\n\n\
         - \"Required\" keys may come from a text style through `styleId` instead of being authored.\n\
         - Root frames (direct children of a page) need numeric `width` and `height`.\n\
         - Text color is `fill`; a container's background is `fill`; corner rounding is `radius`.\n\
         - `update_node` replaces the whole `layout` object: resend `mode` with the other layout keys.\n\
         - `links` targets are root frame IDs checked at the end of the batch; `flowStart` goes on \
         a root frame. `get_design_context` summarizes both as `navigation`.\n\
         - Text wrap collapses ASCII spaces/tabs and preserves LF; NBSP and narrow NBSP remain nonbreaking. Long ordinary words break at graphemes. `textWrap:nowrap` preserves LF and can overflow fixed boxes.\n\
         - Text widths use ceil 1/64px; line y is the alphabetic baseline with font metrics and half-leading. Source text is preserved; lines contain transformed display text.\n\
         - Map letterSpacing to CSS px, textAlign to text-align, textTransform to text-transform using authored text; decoration is continuous (text-decoration-skip-ink:none). Glyph diagnostics prove cmap mappings only, not complex sequence shaping.\n\
         - Paint maps to CSS: `radius` arrays to `border-radius` (overlapping radii scale together), linear/radial `fill` to `linear-gradient`/`radial-gradient` (premultiplied alpha), `shadow` to `box-shadow`. SVG export keeps radii and gradients; shadow and stroke are omitted with `SVG_SHADOW_NOT_EXPORTED`/`SVG_STROKE_NOT_EXPORTED`.\n\
         - Not supported: blur, conic gradients, gradient or shadow on text, shadow in states, `alignSelf`, `space-around`/`space-evenly`. Do not invent keys for them.\n",
    );
    out.push_str(&animations::section());
    out.push_str(instances::section());
    out.push_str("\n## Interaction states\n\n`states` declares hover, pressed, focus and disabled as flat objects. Only fill, opacity, radius and stroke (box kinds) are accepted, with the same token/style references as resting props. No state inherits another or changes children. `update_node` replaces the entire states object; {} clears it. State strokes paint after layout, even inside strokes: resting borders reserve space. Font weight and shadow are not supported. Read statesMode:resolved and map declared variants to CSS pseudo-classes. P07a hover animations still require one explicit keyframe; there is no `to` property or implicit states.hover target.\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_line_and_key_is_published() {
        let doc = text();
        for kind in &NODE_KINDS {
            assert!(doc.contains(&kind_line(kind.as_str(), &allowed_props(kind))));
        }
        for spec in PROPS {
            assert!(doc.contains(&format!("| `{}` |", spec.key)), "{}", spec.key);
        }
    }
}
