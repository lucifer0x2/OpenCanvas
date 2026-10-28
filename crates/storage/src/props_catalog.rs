//! The one table of authored node properties. The validator checks every authored key
//! against it and the server publishes it as `skill://opencanvas/props.md`, so the
//! documented keys cannot drift from what a commit accepts.
use crate::model::NodeKind;

/// Node kinds that may author a property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropScope {
    Any,
    Container,
    Text,
    Image,
}

impl PropScope {
    pub fn allows(self, kind: &NodeKind) -> bool {
        match self {
            Self::Any => true,
            Self::Container => kind.container(),
            Self::Text => *kind == NodeKind::Text,
            Self::Image => *kind == NodeKind::Image,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Any => "all",
            Self::Container => "page, frame, group",
            Self::Text => "text",
            Self::Image => "image",
        }
    }
    /// The validation message when a key is authored on a kind outside this scope.
    pub fn requirement(self) -> &'static str {
        match self {
            Self::Any => "Property is supported on every node kind",
            Self::Container => "Only containers support layout",
            Self::Text => "Text properties require a text node",
            Self::Image => "Image properties require an image node",
        }
    }
}

/// One authored property: where it may appear, what value it takes, and the names
/// agents commonly try instead (used for "did you mean" suggestions).
#[derive(Debug, Clone, Copy)]
pub struct PropSpec {
    pub key: &'static str,
    pub scope: PropScope,
    /// Required on every node of its scope (text content and metrics, image asset).
    pub required: bool,
    pub value: &'static str,
    pub aliases: &'static [&'static str],
}

const fn prop(
    key: &'static str,
    scope: PropScope,
    required: bool,
    value: &'static str,
    aliases: &'static [&'static str],
) -> PropSpec {
    PropSpec {
        key,
        scope,
        required,
        value,
        aliases,
    }
}

use PropScope::{Any, Container, Image, Text};

pub const PROPS: &[PropSpec] = &[
    prop("name", Any, false, "string, at most 256 bytes", &["title"]),
    prop("styleId", Any, false, "ID of a fill style (any kind) or a text style (text nodes only); its values apply under the node's own props", &["style", "textStyle", "fillStyle", "textStyleId", "fillStyleId"]),
    prop("x", Any, false, "number -100000..100000, or {\"tokenId\"} of a number token; used by a free parent", &["left"]),
    prop("y", Any, false, "number -100000..100000, or {\"tokenId\"} of a number token; used by a free parent", &["top"]),
    prop("width", Any, false, "number 0..100000, \"fill\" or \"hug\", or {\"tokenId\"} of a dimension token; root frames need a number; \"fill\" inside a hug parent axis is rejected", &["w"]),
    prop("height", Any, false, "number 0..100000, \"fill\" or \"hug\", or {\"tokenId\"} of a dimension token; root frames need a number; \"fill\" inside a hug parent axis is rejected", &["h"]),
    prop("minWidth", Any, false, "number 0..100000 or dimension token; must not exceed maxWidth", &["min-width"]),
    prop("maxWidth", Any, false, "number 0..100000 or dimension token", &["max-width"]),
    prop("minHeight", Any, false, "number 0..100000 or dimension token; must not exceed maxHeight", &["min-height"]),
    prop("maxHeight", Any, false, "number 0..100000 or dimension token", &["max-height"]),
    prop("layout", Container, false, "object {mode: \"row\"|\"column\"|\"free\", padding?: number or dimension token, gap?: number or dimension token, wrap?: boolean (row only, needs a definite width)}", &["direction", "flexDirection", "flex", "stack", "autoLayout", "mode", "padding", "gap", "wrap"]),
    prop("text", Text, true, "plain string, at most 64 KiB", &["content", "characters", "value", "string"]),
    prop("fontId", Text, true, "ID of an imported font (list_fonts); may come from a text style", &["font", "fontFamily", "family", "fontName"]),
    prop("fontSize", Text, true, "number 1..10000 or dimension token; may come from a text style", &["size", "textSize"]),
    prop("lineHeight", Text, true, "number 1..10000 (pixels) or dimension token; may come from a text style", &["leading", "lineSpacing"]),
    prop("fill", Any, false, "#RGB, #RGBA, #RRGGBB or #RRGGBBAA, a color token {\"tokenId\"} or a fill style {\"styleId\"}; background of shapes and containers, glyph color of text", &["color", "textColor", "background", "backgroundColor", "fillColor", "bg"]),
    prop("opacity", Any, false, "number 0..1 or number token", &["alpha"]),
    prop("clip", Any, false, "boolean; clips children to the node bounds", &["overflow", "clipsContent", "clipContent"]),
    prop("radius", Any, false, "number 0..100000 or dimension token; one radius for all corners", &["cornerRadius", "borderRadius", "corner_radius", "rx"]),
    prop("assetId", Image, true, "ID of an imported image asset (import_asset, list_assets)", &["asset", "src", "image", "imageId", "url"]),
    prop("fit", Image, false, "\"fit\" (stretch to bounds), \"contain\" or \"cover\"", &["objectFit", "scaleMode", "resize"]),
    prop("crop", Image, false, "object {x, y, width, height} normalized 0..1, staying inside the source image", &["cropRect", "sourceRect"]),
];

pub fn prop_spec(key: &str) -> Option<&'static PropSpec> {
    PROPS.iter().find(|spec| spec.key == key)
}

/// Every key a node of this kind may author, in table order.
pub fn allowed_props(kind: &NodeKind) -> Vec<&'static str> {
    PROPS
        .iter()
        .filter(|spec| spec.scope.allows(kind))
        .map(|spec| spec.key)
        .collect()
}

/// Keys that must be present (authored or from a style) on a node of this kind.
pub fn required_props(kind: &NodeKind) -> impl Iterator<Item = &'static str> + '_ {
    PROPS
        .iter()
        .filter(move |spec| spec.required && spec.scope.allows(kind))
        .map(|spec| spec.key)
}

pub const NODE_KINDS: [NodeKind; 6] = [
    NodeKind::Page,
    NodeKind::Frame,
    NodeKind::Group,
    NodeKind::Rectangle,
    NodeKind::Text,
    NodeKind::Image,
];
