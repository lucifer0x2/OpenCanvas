use super::*;
use crate::text_diagnostics::append;

/// One job's real overflow diagnostics: `nodes` overflowing labels with two lines each.
fn job(key: &str, nodes: usize) -> Vec<Value> {
    let mut scene = json!({"diagnostics":[],"provenance":{"sourceSceneKey":key,"viewport":{"width":800,"height":720},
        "fontManifestHash":"f".repeat(64),"assetManifestHash":"a".repeat(64),"theme":"default","state":"default"},
        "nodes":[{"id":"frame","parentId":null,"kind":"frame","clip":true,"bounds":{"x":0,"y":0,"width":100,"height":80}}]});
    for i in 0..nodes {
        let line =
            json!({"x":0,"y":8,"width":35,"inkBounds":{"x":-2,"y":-9,"width":40,"height":12}});
        scene["nodes"].as_array_mut().unwrap().push(json!({"id":format!("label-{i}"),"parentId":"frame","kind":"text",
            "fontId":"font","bounds":{"x":10,"y":10,"width":20,"height":10},"lines":[line.clone(),line]}));
    }
    append(&mut scene, &[]);
    scene["diagnostics"].as_array().unwrap().clone()
}

fn len(issues: &[Value]) -> usize {
    serde_json::to_vec(issues).unwrap().len()
}

fn summaries(issues: &[Value]) -> Vec<&Value> {
    issues.iter().filter(|i| i["code"] == SUMMARY).collect()
}

#[test]
fn fitting_advisories_are_returned_unchanged() {
    let groups = vec![job("a", 2), job("b", 1)];
    let full: Vec<Value> = groups.iter().flatten().cloned().collect();
    assert_eq!(fit(groups, len(&full)), full);
}

#[test]
fn provenance_is_dropped_first_and_items_keep_their_scene_key() {
    let groups = vec![job("a", 2), job("b", 1)];
    let full: Vec<Value> = groups.iter().flatten().cloned().collect();
    let fitted = fit(groups, len(&full) - 1);
    assert!(fitted.iter().all(|i| i.get("provenance").is_none()));
    let items: Vec<_> = fitted.iter().filter(|i| i["code"] == ITEM).collect();
    assert_eq!(
        items,
        full.iter()
            .filter(|i| i["code"] == ITEM)
            .collect::<Vec<_>>()
    );
    for s in summaries(&fitted) {
        assert_eq!(s["compaction"]["step"], "provenance");
        assert_eq!(s["compaction"]["droppedNodes"], 0);
    }
}

#[test]
fn many_jobs_drop_items_round_robin_with_explicit_omissions() {
    let groups: Vec<_> = ["a", "b", "c", "d"].iter().map(|k| job(k, 14)).collect();
    let full: Vec<Value> = groups.iter().flatten().cloned().collect();
    assert!(len(&full) > 30_000, "{}", len(&full));
    let fitted = fit(groups.clone(), 2048);
    assert!(len(&fitted) <= 2048, "{}", len(&fitted));
    assert_eq!(fit(groups, 2048), fitted);
    let summaries = summaries(&fitted);
    assert_eq!(summaries.len(), 4);
    for (s, key) in summaries.iter().zip(["a", "b", "c", "d"]) {
        assert_eq!(s["sourceSceneKey"], key);
        assert_eq!(s["compaction"]["step"], "nodes");
        assert!(s["compaction"]["retrieve"]
            .as_str()
            .unwrap()
            .contains("inspect_layout"));
        let reported = s["reportedNodes"].as_u64().unwrap();
        assert_eq!(reported + s["omittedNodes"].as_u64().unwrap(), 14);
        assert_eq!(
            s["compaction"]["droppedNodes"].as_u64().unwrap(),
            12 - reported
        );
        let items = fitted
            .iter()
            .filter(|i| i["code"] == ITEM && i["sourceSceneKey"] == key);
        assert_eq!(items.count() as u64, reported);
        assert!(reported >= 1, "every job keeps an example: {fitted:?}");
    }
    let item = fitted.iter().find(|i| i["code"] == ITEM).unwrap();
    assert!(item.get("examples").is_none() && item.get("message").is_none());
    assert_eq!(item["outsideLineCount"], 2);
}

#[test]
fn tiny_allowance_collapses_to_one_summary() {
    let fitted = fit(vec![job("a", 14), job("b", 3)], 64);
    assert_eq!(fitted.len(), 1);
    assert_eq!(fitted[0]["code"], SUMMARY);
    assert_eq!(fitted[0]["omittedNodes"], 17);
    assert_eq!(fitted[0]["reportedNodes"], 0);
    assert_eq!(fitted[0]["compaction"]["droppedNodes"], 15);
    assert!(len(&fitted) <= 256, "{}", len(&fitted));
}

#[test]
fn in_place_fit_keeps_other_diagnostics_and_order() {
    let other = json!({"code":"FONT_FALLBACK","message":"m"});
    let mut diagnostics = vec![other.clone()];
    diagnostics.extend(job("a", 12));
    diagnostics.push(other.clone());
    let unchanged = diagnostics.clone();
    fit_in_place(&mut diagnostics, usize::MAX);
    assert_eq!(diagnostics, unchanged);
    fit_in_place(&mut diagnostics, 1024);
    assert_eq!(diagnostics.first(), Some(&other));
    assert_eq!(diagnostics.last(), Some(&other));
    let measured: Vec<Value> = diagnostics[1..diagnostics.len() - 1].to_vec();
    assert!(len(&measured) <= 1024);
    assert_eq!(summaries(&measured)[0]["compaction"]["step"], "nodes");
}
