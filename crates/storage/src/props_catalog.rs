//! The one table of authored node properties. The validator checks every authored key
//! against it and the server publishes it as `skill://opencanvas/props.md`, so the
//! documented keys cannot drift from what a commit accepts.
use crate::model::NodeKind;

/// Node kinds that may author a property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropScope {
    Any,
    /// Every kind except page: nodes that draw pixels.
    Rendered,
    Container,
    Text,
    Image,
    /// Nodes with a painted box: frame, group, rectangle, image.
    Box,
}

impl PropScope {
    pub fn allows(self, kind: &NodeKind) -> bool {
        match self {
            Self::Any => true,
            Self::Rendered => *kind != NodeKind::Page,
            Self::Container => kind.container(),
            Self::Text => *kind == NodeKind::Text,
            Self::Image => *kind == NodeKind::Image,
            Self::Box => !matches!(kind, NodeKind::Page | NodeKind::Text),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Any => "all",
            Self::Rendered => "frame, group, rectangle, text, image",
            Self::Container => "page, frame, group",
            Self::Text => "text",
            Self::Image => "image",
            Self::Box => "frame, group, rectangle, image",
        }
    }
    /// The validation message when a key is authored on a kind outside this scope.
    pub fn requirement(self) -> &'static str {
        match self {
            Self::Any => "Property is supported on every node kind",
            Self::Rendered => "Pages cannot be exported",
            Self::Container => "Only containers support layout",
            Self::Text => "Text properties require a text node",
            Self::Image => "Image properties require an image node",
            Self::Box => "Stroke requires a frame, group, rectangle or image node; wrap text in a frame to give it a border",
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

use PropScope::{Any, Container, Image, Rendered, Text};

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
    prop("layout", Container, false, "object {mode: \"row\"|\"column\"|\"free\", padding?: number, or 1-4 numbers like CSS [all] | [vertical, horizontal] | [top, horizontal, bottom] | [top, right, bottom, left] (each may be a dimension token), gap?: number or dimension token, wrap?: boolean (row only, needs a definite width), align?: cross axis \"start\" (default)|\"center\"|\"end\"|\"stretch\" (stretches children whose cross size is unset), justify?: main axis \"start\" (default)|\"center\"|\"end\"|\"space-between\"}; align/justify need row or column; update_node replaces the whole layout object", &["direction", "flexDirection", "flex", "stack", "autoLayout", "mode", "padding", "gap", "wrap", "alignItems", "justifyContent", "paddingTop", "paddingLeft", "itemSpacing"]),
    prop("text", Text, true, "plain string, at most 64 KiB", &["content", "characters", "value", "string"]),
    prop("fontId", Text, true, "ID of an imported font (list_fonts); may come from a text style", &["font", "fontFamily", "family", "fontName"]),
    prop("fontSize", Text, true, "number 1..10000 or dimension token; may come from a text style", &["size", "textSize"]),
    prop("lineHeight", Text, true, "number 1..10000 (pixels) or dimension token; may come from a text style", &["leading", "lineSpacing"]),
    prop("fill", Any, false, "#RGB, #RGBA, #RRGGBB or #RRGGBBAA, a color token {\"tokenId\"} or a fill style {\"styleId\"}; background of shapes and containers, glyph color of text", &["color", "textColor", "background", "backgroundColor", "fillColor", "bg"]),
    prop("opacity", Any, false, "number 0..1 or number token; on a container, fades the subtree as one group (CSS opacity)", &["alpha"]),
    prop("clip", Any, false, "boolean; clips children to the node bounds", &["overflow", "clipsContent", "clipContent"]),
    prop("radius", Any, false, "number 0..100000 or dimension token; one radius for all corners", &["cornerRadius", "borderRadius", "corner_radius", "rx"]),
    prop("stroke", PropScope::Box, false, "object {color, width?, align?}: color is a hex, color token or fill style, or exactly 4 of them [top, right, bottom, left]; width (default 1) a number 0..100000 or dimension token, or exactly 4 of them; align \"inside\" (default, takes layout space like a CSS border), \"center\" or \"outside\" (painted only). Painted above children", &["border", "borderColor", "borderWidth", "borderTop", "borderRight", "borderBottom", "borderLeft", "strokes", "strokeWidth", "strokeWeight", "strokeColor", "strokeAlign", "outline"]),
    prop("assetId", Image, true, "ID of an imported image asset (import_asset, list_assets)", &["asset", "src", "image", "imageId", "url"]),
    prop("fit", Image, false, "\"fit\" (stretch to bounds, CSS object-fit: fill), \"contain\" (default, whole image inside) or \"cover\" (fill bounds, crop); the same names appear in resolvedProps and scenes", &["objectFit", "scaleMode", "resize"]),
    prop("crop", Image, false, "object {x, y, width, height} normalized 0..1, staying inside the source image", &["cropRect", "sourceRect"]),
    prop("exportSettings", Rendered, false, "array of 0..8 {format?: \"png\", scale?: 0.01..4 | width?: 1..16384 | height?: 1..16384 (only one), suffix?: string} marking the node as an image asset to export (export_nodes); suffix defaults to \"@<scale>x\" when scale is not 1; [] clears it", &["exports", "export", "exportSetting"]),
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
