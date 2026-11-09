use opencanvas_scene_wasm::{frame_keys_json, Snapshot};
use serde_json::{json, Value};

fn fixture() -> Value {
    json!({
        "schemaVersion":4,"projectId":"project","slug":"fixture","name":"Fixture","revision":7,
        "retention":{"oldestCommitRevision":0,"idempotencyPolicy":"lifetime"},"notes":[],
        "tokens":[{"id":"ink","name":"Ink","type":"color",
                "value":{"default":"#111111","themes":{"night":"#eeeeee"}}},
            {"id":"accent","name":"Accent","type":"color","value":"#ff0000"},
            {"id":"unused","name":"Unused","type":"color","value":"#00ff00"},
            {"id":"brand","name":"Brand","type":"color","value":{"aliasId":"accent"}}],
        "styles":[{"id":"copy","name":"Copy","kind":"text","value":{
            "fontId":"font_bundled","fontSize":16,"lineHeight":24,"fill":{"tokenId":"ink"}}}],
        "fonts":[{"id":"font_bundled","family":"Noto","weight":400,"style":"normal",
                "contentHash":"abc","status":"ready"},
            {"id":"other_font","family":"Other","weight":400,"style":"normal",
                "contentHash":"xyz","status":"ready"}],
        "assets":[{"id":"used","contentHash":"def","mediaType":"image/png","byteSize":1,
                "relativeKey":"opaque","status":"ready"},
            {"id":"unused","contentHash":"ghi","mediaType":"image/png","byteSize":1,
                "relativeKey":"opaque2","status":"ready"}],
        "nodes":[
            {"id":"page","kind":"page","parentId":null,"sortKey":0,"props":{}},
            {"id":"master","kind":"frame","parentId":"page","sortKey":0,
                "props":{"width":100,"height":100,"component":{"name":"Tile"}}},
            {"id":"label","kind":"text","parentId":"master","sortKey":0,
                "props":{"text":"Master","styleId":"copy"}},
            {"id":"frame","kind":"frame","parentId":"page","sortKey":1,
                "props":{"width":320,"height":200,"theme":"night"}},
            {"id":"image","kind":"image","parentId":"frame","sortKey":2,
                "props":{"assetId":"used","width":20,"height":20}},
            {"id":"box","kind":"rectangle","parentId":"frame","sortKey":3,
                "props":{"width":10,"height":10,"fill":{"tokenId":"brand"}}},
            {"id":"placed","kind":"instance","parentId":"frame","sortKey":1,
                "props":{"componentId":"master","overrides":{"label":{"text":"Placed"}}}},
            {"id":"page2","kind":"page","parentId":null,"sortKey":1,"props":{}},
            {"id":"elsewhere","kind":"frame","parentId":"page2","sortKey":0,
                "props":{"width":100,"height":100}},
            {"id":"far","kind":"text","parentId":"elsewhere","sortKey":0,
                "props":{"text":"Far","fontId":"other_font","fontSize":12,"lineHeight":16}}
        ]
    })
}

fn keys(snapshot: &Value) -> Vec<Value> {
    let frames = json!([{"frameId":"frame","width":320,"height":200},
        {"frameId":"elsewhere","width":100,"height":100}]);
    let out = frame_keys_json(&snapshot.to_string(), &frames.to_string(), "default").unwrap();
    serde_json::from_str(&out).unwrap()
}

fn digest(snapshot: &Value, index: usize) -> String {
    let keys = keys(snapshot);
    assert!(keys[index]["error"].is_null(), "{:?}", keys[index]);
    keys[index]["digest"].as_str().unwrap().to_owned()
}

fn node<'a>(snapshot: &'a mut Value, id: &str) -> &'a mut Value {
    snapshot["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["id"] == id)
        .unwrap()
}

#[test]
fn every_dependency_of_a_frame_changes_its_key_and_only_its_key() {
    let base = fixture();
    let (frame, other) = (digest(&base, 0), digest(&base, 1));
    assert_eq!(frame.len(), 64);
    type Edit = fn(&mut Value);
    let affecting: [(&str, Edit); 10] = [
        ("subtree prop", |s| {
            node(s, "box")["props"]["width"] = json!(11)
        }),
        ("frame theme", |s| {
            node(s, "frame")["props"]
                .as_object_mut()
                .unwrap()
                .remove("theme");
        }),
        ("token theme value", |s| {
            s["tokens"][0]["value"]["themes"]["night"] = json!("#dddddd")
        }),
        ("aliased token", |s| {
            s["tokens"][1]["value"] = json!("#00ffff")
        }),
        ("style", |s| s["styles"][0]["value"]["fontSize"] = json!(18)),
        ("component master", |s| {
            node(s, "label")["props"]["opacity"] = json!(0.5)
        }),
        ("instance override", |s| {
            node(s, "placed")["props"]["overrides"]["label"]["text"] = json!("Other")
        }),
        ("font face", |s| s["fonts"][0]["weight"] = json!(700)),
        ("asset identity", |s| {
            s["assets"][0]["contentHash"] = json!("new")
        }),
        ("overflow note", |s| {
            s["notes"] = json!([{"id":"n","nodeId":"box","text":"Intentional overflow: yes",
                "author":"a"}])
        }),
    ];
    for (name, edit) in affecting {
        let mut changed = base.clone();
        edit(&mut changed);
        if serde_json::from_value::<Snapshot>(changed.clone()).is_err() {
            panic!("{name}: fixture edit must stay a valid snapshot");
        }
        assert_ne!(digest(&changed, 0), frame, "{name} changes the frame key");
        assert_eq!(
            digest(&changed, 1),
            other,
            "{name} leaves the other page's key"
        );
    }
}

#[test]
fn unrelated_changes_and_the_revision_keep_the_key() {
    let base = fixture();
    let frame = digest(&base, 0);
    type Edit = fn(&mut Value);
    let unrelated: [(&str, Edit); 6] = [
        ("revision", |s| s["revision"] = json!(8)),
        ("other page frame", |s| {
            node(s, "far")["props"]["text"] = json!("Moved")
        }),
        ("unused token", |s| {
            s["tokens"][2]["value"] = json!("#000000")
        }),
        ("unused font", |s| {
            s["fonts"][1]["contentHash"] = json!("new")
        }),
        ("unused asset", |s| {
            s["assets"][1]["contentHash"] = json!("new")
        }),
        // The placed copy overrides this text, so the effective input is unchanged.
        ("overridden master text", |s| {
            node(s, "label")["props"]["text"] = json!("Hidden")
        }),
    ];
    for (name, edit) in unrelated {
        let mut changed = base.clone();
        edit(&mut changed);
        assert_eq!(digest(&changed, 0), frame, "{name} keeps the frame key");
    }
    let mut far = base.clone();
    node(&mut far, "far")["props"]["text"] = json!("Moved");
    assert_ne!(digest(&far, 1), digest(&base, 1));
}

#[test]
fn viewport_and_frame_errors_stay_per_frame() {
    let snapshot = fixture().to_string();
    let frames = json!([{"frameId":"frame","width":320,"height":200},
        {"frameId":"frame","width":390,"height":200},
        {"frameId":"page","width":10,"height":10}]);
    let out: Vec<Value> =
        serde_json::from_str(&frame_keys_json(&snapshot, &frames.to_string(), "default").unwrap())
            .unwrap();
    assert_ne!(out[0]["digest"], out[1]["digest"]);
    assert_eq!(out[2]["error"]["code"], "INVALID_REQUEST");
    assert!(out[2]["digest"].is_null());
    assert!(frame_keys_json(&snapshot, &frames.to_string(), "absent").is_err());
}
