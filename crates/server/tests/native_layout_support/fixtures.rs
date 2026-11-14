//! Existing repo row-wrap fixture plus bounded authored typography/stack fixtures.
use super::workspace;
use serde_json::{json, Value};

pub fn repo_wrap() -> Vec<Value> {
    let result = std::process::Command::new("node").current_dir(workspace())
        .args(["--input-type=module", "-e", "import {wrapOperations} from './spikes/e2e/wrap-design.mjs'; console.log(JSON.stringify(wrapOperations()));"])
        .output().expect("Node is required for the Chromium fixture");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

fn node(id: &str, parent: &str, kind: &str, props: Value) -> Value {
    json!({"op":"create_node","id":id,"parentId":parent,"kind":kind,"props":props})
}

pub fn typography() -> Vec<Value> {
    let mut ops = vec![
        node(
            "typography",
            "page",
            "frame",
            json!({"width":1040,"height":1200,
            "states":{"hover":{"fill":"#edf5ff"}},"layout":{"mode":"column",
                "padding":[20,24],"gap":12,"breakpoints":[{"maxWidth":720,
                    "layout":{"mode":"column","padding":12,"gap":8}}]}}),
        ),
        node(
            "type-cards",
            "typography",
            "group",
            json!({"width":"100%","height":"hug",
            "layout":{"mode":"row","wrap":true,"gap":16}}),
        ),
    ];
    for (index, font) in [
        "font_bundled",
        "inter",
        "lato-regular",
        "lato-bold",
        "lato-italic",
    ]
    .into_iter()
    .enumerate()
    {
        let card = format!("type-card-{index}");
        ops.push(node(
            &card,
            "type-cards",
            "group",
            json!({"width":"fill","minWidth":240,
            "height":"hug","layout":{"mode":"column","padding":12,"gap":8}}),
        ));
        for (label, text, size, height, tracking, transform, align) in [
            ("heading", "Tiếng Việt office AV ß ﬃ", 19.25, 26.5, 0.25, "uppercase", "center"),
            ("body", "Một thiết kế có thể xuống dòng.\r\nDòng hai\t AV office a\u{301} n\u{323} và kết thúc.", 16.0, 24.0, 0.0, "none", "left"),
            ("caption", "hello-world tiếng việt", 13.25, 17.5, -0.125, "capitalize", "right"),
        ] {
            ops.push(node(&format!("{card}-{label}"), &card, "text", json!({"text":text,
                "width":if label == "caption" { "75%" } else { "fill" },"height":"hug",
                "fontId":font,"fontSize":size,"lineHeight":height,"letterSpacing":tracking,
                "textTransform":transform,"textAlign":align})));
        }
    }
    ops.extend([
        node("hug-row", "typography", "group", json!({"width":"hug","height":"hug",
            "layout":{"mode":"row","gap":7,"padding":3,"align":"center"}})),
        node("hug-text", "hug-row", "text", json!({"text":"AV office Việt","fontId":"inter",
            "fontSize":14,"lineHeight":18,"width":"hug","height":"hug"})),
        node("fixed-child", "hug-row", "rectangle", json!({"width":13,"height":21})),
        node("clip-box", "typography", "group", json!({"width":120,"height":24,"clip":true,"layout":{"mode":"free"}})),
        node("clipped-text", "clip-box", "text", json!({"x":130,"y":30,"width":150,"height":"hug",
            "text":"Clipped but measured","fontId":"lato-italic","fontSize":16,"lineHeight":23})),
        node("transparent-text", "typography", "text", json!({"width":"50%","height":"hug","opacity":0,
            "text":"Invisible but measured Việt","fontId":"font_bundled","fontSize":16,"lineHeight":24,"textAlign":"center"})),
        node("mixed-script", "typography", "text", json!({"width":"fill","height":"hug",
            "text":"Việt a\u{301} n\u{323} ΟΣ ος Alpha","fontId":"font_bundled","fontSize":21,"lineHeight":29})),
    ]);
    ops
}

pub fn missing_control() -> Vec<Value> {
    vec![
        node(
            "missing-control",
            "page",
            "frame",
            json!({"width":1040,"height":1200,"layout":{"mode":"column"}}),
        ),
        node(
            "missing-text",
            "missing-control",
            "text",
            json!({"width":"fill","height":"hug",
            "text":"Missing ⌘ 🦄 一","fontId":"font_bundled","fontSize":20,"lineHeight":28}),
        ),
    ]
}

/// A long page: the root height hugs wrapped text, so it changes with the viewport width.
pub fn long_page() -> Vec<Value> {
    let copy = "Một trang dài có chiều cao theo nội dung, xuống dòng theo bề rộng. ".repeat(6);
    vec![
        node(
            "long-page",
            "page",
            "frame",
            json!({"width":1040,"height":"hug","layout":{"mode":"column","padding":[20,24],
                "gap":12,"breakpoints":[{"maxWidth":720,"layout":{"mode":"column","padding":12}}]}}),
        ),
        node(
            "long-copy",
            "long-page",
            "text",
            json!({"width":"fill","height":"hug","text":copy,"fontId":"inter","fontSize":18,
                "lineHeight":27}),
        ),
        node(
            "long-footer",
            "long-page",
            "rectangle",
            json!({"width":"fill","height":48}),
        ),
    ]
}
