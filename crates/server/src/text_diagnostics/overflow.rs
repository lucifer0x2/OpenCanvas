//! Evidence from immutable measured lines. Advance width is never called glyph ink.
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

pub(crate) fn append(scene: &mut Value, intentional: &[String]) {
    let Some(nodes) = scene["nodes"].as_array() else {
        return;
    };
    let by_id: HashMap<_, _> = nodes
        .iter()
        .filter_map(|n| Some((n["id"].as_str()?, n)))
        .collect();
    let intentional: HashSet<_> = intentional.iter().map(String::as_str).collect();
    let mut issues = Vec::new();
    for node in nodes.iter().filter(|n| n["kind"] == "text") {
        let id = node["id"].as_str().unwrap_or("");
        let Some(own) = rect(&node["bounds"]) else {
            continue;
        };
        let mut clips = Vec::new();
        let mut current = Some(node);
        let mut hidden = false;
        while let Some(ancestor) = current {
            hidden |= ancestor["opacity"].as_f64() == Some(0.);
            if ancestor["clip"] == true {
                if let Some(bounds) = rect(&ancestor["bounds"]) {
                    clips.push((ancestor["id"].clone(), bounds));
                }
            }
            current = ancestor["parentId"]
                .as_str()
                .and_then(|p| by_id.get(p).copied());
        }
        if hidden {
            continue;
        }
        let mut examples = Vec::new();
        let mut count = 0;
        let mut has_ink = false;
        for (index, line) in node["lines"].as_array().into_iter().flatten().enumerate() {
            let (Some(x), Some(y), Some(width)) = (
                finite(&line["x"]),
                finite(&line["y"]),
                finite(&line["width"]),
            ) else {
                continue;
            };
            let advance = [own[0] + x, own[1] + y, width, 0.];
            let ink = rect(&line["inkBounds"])
                .map(|r| [advance[0] + r[0], advance[1] + r[1], r[2], r[3]]);
            has_ink |= ink.is_some();
            let advance_outside = outside_x(advance, own);
            let ink_outside = ink.is_some_and(|r| outside(r, own));
            let clip_ids: Vec<_> = clips
                .iter()
                .filter(|(_, b)| outside_x(advance, *b) || ink.is_some_and(|r| outside(r, *b)))
                .map(|(id, _)| id)
                .collect();
            if !advance_outside && !ink_outside && clip_ids.is_empty() {
                continue;
            }
            count += 1;
            if examples.len() < 2 {
                examples.push(json!({"lineIndex":index,"advance":{"x":advance[0],"baselineY":advance[1],"width":width,"outsideOwnBox":advance_outside},
                    "ink":ink.map(|r| json!({"x":r[0],"y":r[1],"width":r[2],"height":r[3],"outsideOwnBox":ink_outside})),"clipNodeIds":clip_ids}));
            }
        }
        if count == 0 {
            continue;
        }
        issues.push(json!({"code":"TEXT_CONTENT_OUTSIDE_BOX","severity":if intentional.contains(id) {"info"}else{"warning"},
            "nodeId":id,"fontId":node["fontId"],"scope":"complete-measured-scene-before-pagination",
            "intentionalOverflow":intentional.contains(id),"outsideLineCount":count,"examples":examples,"box":node["bounds"],
            "inkEvidence":if has_ink {"measured"}else{"unavailable"},
            "provenance":scene["provenance"],
            "message":"Measured text exceeds its box or a clip. Advance is layout width; ink is measured glyph extent when available. Overflow can be intentional; this advisory never changes authoring."}));
    }
    let total = issues.len();
    issues.truncate(12);
    if total > 12 {
        issues.push(json!({"code":"TEXT_OVERFLOW_SUMMARY","scope":"complete-measured-scene-before-pagination","omittedNodes":total-12}));
    }
    if let Some(diagnostics) = scene["diagnostics"].as_array_mut() {
        diagnostics.extend(issues);
    }
}
fn finite(v: &Value) -> Option<f64> {
    v.as_f64().filter(|n| n.is_finite())
}
fn rect(v: &Value) -> Option<[f64; 4]> {
    Some([
        finite(&v["x"])?,
        finite(&v["y"])?,
        finite(&v["width"])?,
        finite(&v["height"])?,
    ])
}
fn outside_x(r: [f64; 4], b: [f64; 4]) -> bool {
    r[0] < b[0] - 0.5 || r[0] + r[2] > b[0] + b[2] + 0.5
}
fn outside(r: [f64; 4], b: [f64; 4]) -> bool {
    outside_x(r, b) || r[1] < b[1] - 0.5 || r[1] + r[3] > b[1] + b[3] + 0.5
}
