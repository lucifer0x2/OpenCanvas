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
    /// Every kind except page.
    Layer,
    /// Frames only.
    Frame,
    Container,
    Text,
    Image,
    Instance,
    /// Nodes with a painted box: frame, group, rectangle, image.
    Box,
}

impl PropScope {
    pub fn allows(self, kind: &NodeKind) -> bool {
        match self {
            Self::Any => true,
            Self::Instance => *kind == NodeKind::Instance,
            Self::Rendered => *kind != NodeKind::Page,
            Self::Layer => *kind != NodeKind::Page,
            Self::Frame => *kind == NodeKind::Frame,
            Self::Container => kind.container(),
            Self::Text => *kind == NodeKind::Text,
            Self::Image => *kind == NodeKind::Image,
            Self::Box => !matches!(kind, NodeKind::Page | NodeKind::Text),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Any => "all",
            Self::Instance => "instance",
            Self::Rendered => "frame, group, rectangle, text, image",
            Self::Layer => "frame, group, rectangle, text, image",
            Self::Frame => "frame",
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
            Self::Instance => "Only instances support componentId and overrides",
            Self::Rendered => {
                "Pages cannot be exported or animated; put exportSettings and animations on a node inside a frame"
            }
            Self::Layer => "Pages cannot carry links or states; put them on a node inside a frame",
            Self::Frame => "Only frames support flowStart",
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

use PropScope::{Any, Container, Frame, Image, Layer, Rendered, Text};

pub const PROPS: &[PropSpec] = &[
    prop("component", Frame, false, "object {name: string}; reusable master, no links or animations in its subtree", &[]),
    prop("componentId", PropScope::Instance, true, "ID of a component frame", &[]),
    prop("overrides", PropScope::Instance, false, "object keyed by master descendant IDs; partial text, fill, visible, styleId, assetId, name", &[]),
    prop("visible", Any, false, "boolean; false hides the node and its descendants", &[]),
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
    prop("fontId", Text, false, "ID of an imported font (list_fonts), or choose fontFamily; one font selector is required; may come from a text style", &["font", "fontName"]),
    prop("fontFamily", Text, false, "exact project font family; choose fontId or fontFamily; default weight400/style normal; may come from text style", &["family", "font-family", "typeface"]),
    prop("fontWeight", Text, false, "integer 100..900 or number token; selects an existing exact face, no synthetic bold", &["weight", "font-weight", "bold"]),
    prop("fontStyle", Text, false, "normal or italic; selects an existing exact face", &["italic", "font-style", "slant"]),
    prop("fontSize", Text, true, "number 1..10000 or dimension token; may come from a text style", &["size", "textSize"]),
    prop("lineHeight", Text, true, "number 1..10000 (pixels) or dimension token; may come from a text style", &["leading", "lineSpacing"]),
    prop("textWrap", Text, false, "wrap (default) or nowrap; LF still forces a line; hug+nowrap is max-content", &["whiteSpace", "wordWrap", "textAutoResize", "singleLine"]),
    prop("letterSpacing", Text, false, "signed pixels -1000..1000 or number token; default0; em must be converted using fontSize", &["tracking", "letter-spacing", "charSpacing", "characterSpacing"]),
    prop("textAlign", Text, false, "left (default), center or right; aligns measured lines in the box", &["textAlignHorizontal", "horizontalAlign", "text-align", "alignText"]),
    prop("textDecoration", Text, false, "none (default), underline or line-through; continuous ink (no skip-ink)", &["decoration", "textDecorationLine", "underline", "strikethrough"]),
    prop("textTransform", Text, false, "none (default), uppercase, lowercase or capitalize; preserves authored source text", &["textCase", "case", "text-transform", "uppercase", "capitalize"]),
    prop("fill", Any, false, "solid hex color, color token {\"tokenId\"} or fill style {\"styleId\"}; boxes also accept {type:\"linear\",angle?:180,stops:[[position,color],...]}, or {type:\"radial\",center?:[0.5,0.5],radius?:number|[rx,ry],stops}; 2..16 ordered stops 0..1, angle -360..360, center -10..10, radius >0..100000 px (omitted: farthest-corner ellipse). Text fill is solid; fill animation requires a solid resting fill", &["color", "textColor", "background", "backgroundColor", "fillColor", "bg", "gradient", "backgroundImage", "fills", "linearGradient", "radialGradient"]),
    prop("opacity", Any, false, "number 0..1 or number token; on a container, fades the subtree as one group (CSS opacity)", &["alpha"]),
    prop("clip", Any, false, "boolean; clips children to the node bounds", &["overflow", "clipsContent", "clipContent"]),
    prop("radius", Any, false, "number 0..100000 or dimension token, or 1–4 values with CSS shorthand [top-left,top-right,bottom-right,bottom-left]; each value can be a dimension token; overlapping radii scale together like CSS", &["cornerRadius", "borderRadius", "corner_radius", "rx"]),
    prop("stroke", PropScope::Box, false, "object {color, width?, align?}: color is a solid hex, color token or solid fill style, or 1–4 in CSS shorthand [top,right,bottom,left]; width (default 1) a number 0..100000 or dimension token, or 1–4 values; align \"inside\" (default, takes layout space like a CSS border), \"center\" or \"outside\" (painted only). Painted above children", &["border", "borderColor", "borderWidth", "borderTop", "borderRight", "borderBottom", "borderLeft", "strokes", "strokeWidth", "strokeWeight", "strokeColor", "strokeAlign", "outline"]),
    prop("shadow", PropScope::Box, false, "array of 0..8 CSS box-shadow layers {x?:0,y?:0,blur?:0,spread?:0,color,inset?:false}; first layer on top. x/y -10000..10000 and spread -1000..1000 accept number tokens; blur 0..1000 accepts dimension tokens; color accepts solid hex/color token/solid fill style. Wrap text in a frame for a box shadow. Interaction state shadow is unsupported", &["boxShadow", "box-shadow", "shadows", "dropShadow", "innerShadow", "effects", "elevation"]),
    prop("assetId", Image, true, "ID of an imported image asset (import_asset, list_assets)", &["asset", "src", "image", "imageId", "url"]),
    prop("fit", Image, false, "\"fit\" (stretch to bounds, CSS object-fit: fill), \"contain\" (default, whole image inside) or \"cover\" (fill bounds, crop); the same names appear in resolvedProps and scenes", &["objectFit", "scaleMode", "resize"]),
    prop("crop", Image, false, "object {x, y, width, height} normalized 0..1, staying inside the source image", &["cropRect", "sourceRect"]),
    prop("exportSettings", Rendered, false, "array of 0..8 {format?: \"png\" | \"svg\", scale?: 0.01..4 | width?: 1..16384 | height?: 1..16384 (only one; none for svg), suffix?: string, svgIdAttribute?: boolean (svg only)} marking the node as an image asset to export (export_nodes); suffix defaults to \"@<scale>x\" when scale is not 1; [] clears it", &["exports", "export", "exportSetting"]),
    prop("links", Layer, false, "array of {to?, on?: \"click\" (default) | \"hover\", action?: \"navigate\" (default) | \"overlay\" | \"back\" | \"close\"}; to = a root frame ID, required for navigate/overlay, absent for back/close; one link per on; hover needs overlay; [] = none", &["reactions", "interactions", "prototype", "onClick", "href", "link", "navigation"]),
    prop("flowStart", Frame, false, "flow name 1–64 bytes, unique in the project; marks this root frame as the first screen of that flow", &["startingPoint", "flowStartingPoint", "start", "entry", "initialRoute"]),
    prop("animations", Rendered, false, "array of at most 8 {name, on?, duration, delay?, easing?, iterations?, direction?, fillMode?, keyframes}; keyframes animate opacity, translateX/Y, scale, rotate and fill from the authored (resting) props; see Animations below; [] = none", &["animation", "motion", "transitions", "transition", "keyframes"]),
    prop("states", Layer, false, "object of hover|pressed|focus|disabled flat visual overrides: fill, opacity, radius and stroke (box kinds only); refs resolve per theme; {} clears variants; no inheritance or layout changes", &[]),
];

pub fn prop_spec(key: &str) -> Option<&'static PropSpec> {
    PROPS.iter().find(|spec| spec.key == key)
}

/// Every key a node of this kind may author, in table order.
pub fn allowed_props(kind: &NodeKind) -> Vec<&'static str> {
    if *kind == NodeKind::Instance {
        return crate::instances::INSTANCE_KEYS.to_vec();
    }
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

pub const NODE_KINDS: [NodeKind; 7] = [
    NodeKind::Page,
    NodeKind::Frame,
    NodeKind::Group,
    NodeKind::Rectangle,
    NodeKind::Text,
    NodeKind::Image,
    NodeKind::Instance,
];
