//! Row/column stack rules mapped onto Taffy flexbox, CSS style.
use crate::model::{LayoutRule, Props, Sizing};
use taffy::prelude::*;

pub(crate) const ALIGNS: [&str; 4] = ["start", "center", "end", "stretch"];
pub(crate) const JUSTIFIES: [&str; 4] = ["start", "center", "end", "space-between"];

/// The parent context a child is laid out in.
#[derive(Clone, Copy)]
pub(crate) struct Parent<'a> {
    pub mode: &'a str,
    pub align: &'a str,
}

impl Parent<'_> {
    pub(crate) const ROOT: Parent<'static> = Parent {
        mode: "free",
        align: "start",
    };
}

/// Padding (plus `insets` taken by the box itself, [t, r, b, l]), gap, wrap and alignment.
pub(crate) fn apply(style: &mut Style, layout: &LayoutRule, insets: [f32; 4]) {
    if layout.wrap {
        style.flex_wrap = FlexWrap::Wrap;
        style.align_content = AlignContent::START;
    }
    let [t, r, b, l] = layout.padding.edges().unwrap_or_default();
    style.padding = Rect {
        top: length(t + insets[0]),
        right: length(r + insets[1]),
        bottom: length(b + insets[2]),
        left: length(l + insets[3]),
    };
    style.gap = Size {
        width: length(layout.gap),
        height: length(layout.gap),
    };
    style.align_items = match layout.align.as_deref() {
        Some("center") => AlignItems::CENTER,
        Some("end") => AlignItems::END,
        Some("stretch") => AlignItems::STRETCH,
        _ => AlignItems::START,
    };
    style.justify_content = match layout.justify.as_deref() {
        Some("center") => JustifyContent::CENTER,
        Some("end") => JustifyContent::END,
        Some("space-between") => JustifyContent::SPACE_BETWEEN,
        _ => JustifyContent::START,
    };
}

/// The effective cross-axis alignment children of `layout` receive.
pub(crate) fn align(layout: Option<&LayoutRule>) -> &str {
    layout.and_then(|l| l.align.as_deref()).unwrap_or("start")
}

/// A `hug` child keeps its content size in a stretching stack (CSS `fit-content`).
pub(crate) fn self_align(child: &Props, parent: Parent) -> Option<AlignSelf> {
    let cross = match parent.mode {
        "row" => &child.height,
        "column" => &child.width,
        _ => return None,
    };
    (parent.align == "stretch" && matches!(cross, Some(Sizing::Rule(r)) if r == "hug"))
        .then_some(AlignSelf::START)
}
