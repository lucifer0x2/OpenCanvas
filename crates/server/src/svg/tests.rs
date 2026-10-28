//! `scene_to_svg` on a fixture scene, parsed back with roxmltree; no browser.
use super::{scene_to_svg, Image};
use serde_json::{json, Value};
use std::collections::HashMap;

fn node(id: &str, parent: Option<&str>, kind: &str, b: [f64; 4], extra: Value) -> Value {
    let mut node = json!({"id":id,"parentId":parent,"kind":kind,"bounds":{"x":b[0],"y":b[1],"width":b[2],"height":b[3]},"lines":[],"fontId":null,"fontSize":16,"lineHeight":24,"fill":"transparent","assetId":null,"fit":"contain","opacity":1,"clip":false,"radius":0,"crop":null});
    node.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    node
}

/// Frame fill, a clipping faded group with an 8-digit hex fill and a rounded rect inside, text
/// with two lines and a cropped rounded `cover` image.
fn scene() -> Value {
    json!({"provenance":{"viewport":{"width":120,"height":80}},"fonts":[{"id":"f1","family":"Noto Sans","weight":600,"style":"italic"}],"assets":[],"nodes":[
        node("frame", None, "frame", [0.0, 0.0, 120.0, 80.0], json!({"fill":"#ffffff"})),
        node("group", Some("frame"), "group", [10.0, 10.0, 60.0, 40.0], json!({"fill":"#11223380","clip":true,"radius":8,"opacity":0.5})),
        node("rect", Some("group"), "rectangle", [12.0, 12.0, 30.0, 20.0], json!({"fill":"#EF4444","radius":6,"opacity":0.8})),
        node("text", Some("frame"), "text", [70.25, 10.0, 40.0, 40.0], json!({"fontId":"f1","fontSize":12,"fill":"#182230","lines":[{"text":"a<b & \"c\"","x":0,"y":14,"width":30},{"text":" two","x":1.5,"y":30.125,"width":20}]})),
        node("image", Some("frame"), "image", [10.0, 55.0, 40.0, 20.0], json!({"assetId":"a1","fit":"cover","radius":4,"crop":{"x":0.25,"y":0,"width":0.5,"height":1}})),
    ]})
}

fn images() -> HashMap<String, Image> {
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    png.extend([0, 0, 0, 200, 0, 0, 0, 100]);
    HashMap::from([("a1".into(), Image::new("image/png", png).unwrap())])
}

fn svg(id_attribute: bool) -> String {
    scene_to_svg(&scene(), &images(), id_attribute)
        .unwrap()
        .markup
}

fn named<'a>(doc: &'a roxmltree::Document, tag: &str) -> Vec<roxmltree::Node<'a, 'a>> {
    doc.descendants()
        .filter(|n| n.tag_name().name() == tag)
        .collect()
}

fn in_clip_path(node: &roxmltree::Node) -> bool {
    node.ancestors().any(|a| a.tag_name().name() == "clipPath")
}

#[test]
fn root_and_element_counts_follow_the_scene() {
    let markup = svg(false);
    let doc = roxmltree::Document::parse(&markup).unwrap();
    let root = doc.root_element();
    assert_eq!(
        root.tag_name().namespace(),
        Some("http://www.w3.org/2000/svg")
    );
    assert_eq!(root.attribute("width"), Some("120"));
    assert_eq!(root.attribute("height"), Some("80"));
    assert_eq!(root.attribute("viewBox"), Some("0 0 120 80"));
    let painted: Vec<_> = named(&doc, "rect")
        .into_iter()
        .filter(|n| !in_clip_path(n))
        .collect();
    assert_eq!(painted.len(), 3, "frame, group and rect have fills");
    assert_eq!(named(&doc, "text").len(), 2);
    assert_eq!(named(&doc, "image").len(), 1);
    assert_eq!(
        named(&doc, "clipPath").len(),
        2,
        "group clip + image radius"
    );
    // The rect inside the clipping group is drawn under that group's clip.
    let rect = painted
        .iter()
        .find(|n| n.attribute("rx") == Some("6"))
        .unwrap();
    assert!(rect.ancestors().any(|a| a.attribute("clip-path").is_some()));
    assert_eq!(rect.attribute("opacity"), Some("0.8"));
    let group_opacity = rect
        .ancestors()
        .find_map(|a| a.attribute("opacity").filter(|_| a.has_tag_name("g")));
    assert_eq!(group_opacity, Some("0.5"), "the group composites once");
}

#[test]
fn text_lines_sit_at_bounds_plus_line_and_round_trip() {
    let markup = svg(false);
    let doc = roxmltree::Document::parse(&markup).unwrap();
    let texts = named(&doc, "text");
    let at = |n: &roxmltree::Node, key| n.attribute(key).unwrap().parse::<f64>().unwrap();
    assert!((at(&texts[0], "x") - 70.25).abs() < 0.001);
    assert!((at(&texts[0], "y") - 24.0).abs() < 0.001);
    assert!((at(&texts[1], "x") - 71.75).abs() < 0.001);
    assert!((at(&texts[1], "y") - 40.125).abs() < 0.001);
    assert_eq!(texts[0].text(), Some("a<b & \"c\""));
    assert_eq!(texts[1].text(), Some(" two"));
    assert_eq!(
        texts[0].attribute("font-family"),
        Some("'Noto Sans', sans-serif")
    );
    assert_eq!(texts[0].attribute("font-weight"), Some("600"));
    assert_eq!(texts[0].attribute("font-style"), Some("italic"));
    assert_eq!(texts[0].attribute("font-size"), Some("12"));
}

#[test]
fn eight_digit_hex_splits_into_fill_opacity() {
    let markup = svg(false);
    assert!(
        markup.contains("fill=\"#112233\" fill-opacity=\"0.502\""),
        "{markup}"
    );
    assert!(markup.contains("fill=\"#ef4444\""), "{markup}");
    let long_hex = markup.match_indices('#').any(|(i, _)| {
        let rest = &markup.as_bytes()[i + 1..];
        rest.len() > 8 && rest[..8].iter().all(u8::is_ascii_hexdigit)
    });
    assert!(!long_hex, "{markup}");
}

#[test]
fn cropped_image_uses_a_viewbox_of_natural_pixels() {
    let markup = svg(false);
    let doc = roxmltree::Document::parse(&markup).unwrap();
    let image = &named(&doc, "image")[0];
    let viewport = image.parent_element().unwrap();
    assert_eq!(viewport.attribute("viewBox"), Some("50 0 100 100"));
    assert_eq!(
        viewport.attribute("preserveAspectRatio"),
        Some("xMidYMid slice")
    );
    assert_eq!(image.attribute("width"), Some("200"));
    assert!(image
        .attribute("href")
        .unwrap()
        .starts_with("data:image/png;base64,"));
}

#[test]
fn id_attribute_marks_each_node_once() {
    let ids = |markup: &str| -> Vec<String> {
        let doc = roxmltree::Document::parse(markup).unwrap();
        doc.descendants()
            .filter_map(|n| n.attribute("id"))
            .filter(|id| !id.starts_with("oc-clip-"))
            .map(str::to_owned)
            .collect()
    };
    assert!(ids(&svg(false)).is_empty());
    let mut marked = ids(&svg(true));
    marked.sort();
    assert_eq!(marked, ["frame", "group", "image", "rect", "text"]);
}

#[test]
fn small_icon_stays_small_and_flags_text() {
    let icon = json!({"fonts":[{"id":"f1","family":"Noto Sans","weight":400,"style":"normal"}],"nodes":[
        node("icon", None, "frame", [0.0, 0.0, 24.0, 24.0], json!({"fill":"#2563eb","radius":6})),
        node("glyph", Some("icon"), "text", [4.0, 2.0, 16.0, 20.0], json!({"fontId":"f1","fontSize":14,"fill":"#ffffff","lines":[{"text":"A","x":3,"y":15,"width":9}]})),
    ]});
    let svg = scene_to_svg(&icon, &HashMap::new(), false).unwrap();
    assert!(
        svg.markup.len() <= 600,
        "{} bytes: {}",
        svg.markup.len(),
        svg.markup
    );
    assert!(svg.has_text);
    roxmltree::Document::parse(&svg.markup).unwrap();
    let shapes = scene_to_svg(
        &json!({"nodes":[scene()["nodes"][0].clone()]}),
        &HashMap::new(),
        false,
    )
    .unwrap();
    assert!(!shapes.has_text);
}

#[test]
fn missing_image_bytes_fail_instead_of_a_partial_svg() {
    let error = scene_to_svg(&scene(), &HashMap::new(), false)
        .err()
        .unwrap();
    assert_eq!(error.0.code, "RESOURCE_NOT_READY");
}

#[test]
fn xml_invalid_control_characters_are_dropped() {
    let mut scene = scene();
    scene["nodes"][3]["lines"][0]["text"] = json!("ctrl:\u{1}\u{8}\u{1b}\u{fffe}\tok\u{7f}");
    let markup = scene_to_svg(&scene, &images(), false).unwrap().markup;
    let doc = roxmltree::Document::parse(&markup).unwrap();
    assert_eq!(named(&doc, "text")[0].text(), Some("ctrl:\tok\u{7f}"));
}
