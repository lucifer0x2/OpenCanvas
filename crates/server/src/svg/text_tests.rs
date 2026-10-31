//! Node opacity of leaf text: one `<g opacity>` when the node's own elements overlap
//! (decoration, lines whose ink meets), per-element opacity otherwise.
use super::scene_to_svg;
use serde_json::{json, Value};
use std::collections::HashMap;

fn text(extra: Value) -> String {
    let mut text = json!({"id":"t","parentId":"f","kind":"text","bounds":{"x":10,"y":10,"width":200,"height":100},"fontId":null,"fontSize":40,"fill":"#000000","opacity":0.5,
        "lines":[{"text":"HHHH","x":0,"y":40,"width":110,"inkBounds":{"x":2,"y":-29,"width":106,"height":29}}]});
    text.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    let scene = json!({"nodes":[
        {"id":"f","parentId":null,"kind":"frame","bounds":{"x":0,"y":0,"width":220,"height":120},"fill":"#ffffff"},
        text,
    ]});
    let markup = scene_to_svg(&scene, &HashMap::new(), false).unwrap().markup;
    roxmltree::Document::parse(&markup).unwrap();
    markup
}

/// The markup after the frame's own `<rect>`, i.e. the text node's elements.
fn body(markup: &str) -> &str {
    let start = markup.find("fill=\"#ffffff\"/>").unwrap() + "fill=\"#ffffff\"/>".len();
    &markup[start..markup.len() - "</svg>".len()]
}

#[test]
fn decorated_translucent_text_composites_glyphs_and_decoration_once() {
    let markup = text(json!({"decoration":{"line":"line-through","offset":-12,"thickness":3}}));
    let own = body(&markup);
    assert!(own.starts_with("<g opacity=\"0.5\"><text "), "{own}");
    assert!(own.ends_with("/></g>"), "{own}");
    let inner = &own["<g opacity=\"0.5\">".len()..];
    assert!(
        !inner.contains(" opacity="),
        "no per-element opacity: {own}"
    );
    assert!(own.contains("<rect x=\"10\" y=\"38\" width=\"110\" height=\"3\" fill=\"#000000\"/>"));
    // A translucent colour still paints glyph and decoration separately, as CSS does.
    let markup = text(
        json!({"fill":"#00000080","decoration":{"line":"underline","offset":4,"thickness":2}}),
    );
    assert_eq!(body(&markup).matches("fill-opacity=\"0.502\"").count(), 2);
}

#[test]
fn undecorated_single_line_and_separate_lines_keep_per_element_opacity() {
    let single = text(json!({}));
    assert!(!single.contains("<g"), "{single}");
    assert!(body(&single).starts_with("<text x=\"10\" y=\"50\" font-family=\"sans-serif\" font-size=\"40\" letter-spacing=\"0\" style=\"white-space:pre;font-kerning:normal\" fill=\"#000000\" opacity=\"0.5\" xml:space=\"preserve\">HHHH</text>"), "{single}");
    // Negative tracking stays one `<text>`, whose own opacity composites its glyphs once.
    assert!(!text(json!({"letterSpacing":-20})).contains("<g"));
    let ink = json!({"x":0,"y":-29,"width":100,"height":38});
    let line = |y: f64| json!({"text":"gjpq","x":0,"y":y,"width":100,"inkBounds":ink});
    let apart = text(json!({"lines":[line(40.0), line(90.0)]}));
    assert!(!apart.contains("<g") && apart.matches(" opacity=\"0.5\"").count() == 2);
    // Descenders of line one (to y 49) reach the ascent of line two (from y 46).
    assert!(text(json!({"lines":[line(40.0), line(75.0)]})).contains("<g opacity=\"0.5\">"));
    let bare = json!([{"text":"a","x":0,"y":40,"width":10},{"text":"b","x":0,"y":90,"width":10}]);
    assert!(text(json!({ "lines": bare })).contains("<g opacity=\"0.5\">"));
}
