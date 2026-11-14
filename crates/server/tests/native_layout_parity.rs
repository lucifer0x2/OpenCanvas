//! Task10 full-scene geometry acceptance; ink/raster parity is explicitly separate.
#[path = "native_support/mcp.rs"]
#[allow(dead_code)]
mod mcp;
mod native_layout_support;
use native_layout_support::{
    compare::{self, Comparison},
    fixtures, fonts, runtime, Server,
};
use opencanvas_server::RenderBackend;
use opencanvas_storage::{Storage, Store};
use serde_json::{json, Value};

fn descendants(ops: &[Value], frame: &str) -> Vec<String> {
    let mut ids = vec![frame.to_owned()];
    for op in ops {
        if op["op"] == "create_node"
            && op["parentId"]
                .as_str()
                .is_some_and(|p| ids.iter().any(|id| id == p))
        {
            ids.push(op["id"].as_str().unwrap().to_owned());
        }
    }
    ids
}

fn metadata(browser: &Value, native: &Value) {
    assert_ne!(
        browser["sceneKey"], native["sceneKey"],
        "backend cache isolation"
    );
    assert_eq!(
        browser["scene"]["provenance"]["browserVersion"],
        "156.0.8078.4"
    );
    assert_eq!(native["scene"]["provenance"]["engine"], "native");
    assert!(native["scene"]["provenance"]
        .get("browserVersion")
        .is_none());
    for axis in ["width", "height"] {
        assert_eq!(
            browser["scene"]["provenance"]["viewport"][axis].as_f64(),
            native["scene"]["provenance"]["viewport"][axis].as_f64()
        );
    }
    for field in [
        "projectId",
        "revision",
        "frameId",
        "theme",
        "state",
        "sceneVersion",
        "rendererVersion",
        "assetManifestHash",
    ] {
        assert_eq!(
            browser["scene"]["provenance"][field], native["scene"]["provenance"][field],
            "provenance {field}"
        );
    }
    // Font URL and its derived manifest hash legitimately differ between the two ports.
    let mut a = browser["scene"]["fonts"].clone();
    let mut b = native["scene"]["fonts"].clone();
    for fonts in [&mut a, &mut b] {
        for font in fonts.as_array_mut().unwrap() {
            font.as_object_mut().unwrap().remove("url");
        }
    }
    assert_eq!(a, b, "same font bytes, metadata, weights and styles");
    eprintln!("PROVENANCE chromium={} native={} readinessChromium={} readinessNative={} paintErrorChromium={} paintErrorNative={}", browser["provenance"], native["provenance"], browser["readiness"], native["readiness"], browser["error"], native["error"]);
}

#[tokio::test]
async fn complete_scene_geometry_is_exact_between_real_chromium_and_node_free_native() {
    let checked_sources = runtime::check();
    let root = tempfile::tempdir().unwrap();
    let store: Storage = Store::open(root.path().join("data")).unwrap().into();
    let chromium = Server::start(store.clone(), RenderBackend::Chromium, root.path()).await;
    let native = Server::start(store, RenderBackend::Rust, root.path()).await;
    assert_ne!(
        chromium.state.config.pipeline_identity,
        native.state.config.pipeline_identity
    );
    eprintln!(
        "PIPELINE chromium={} native={}",
        chromium.state.config.pipeline_identity, native.state.config.pipeline_identity
    );
    let project = chromium.project().await;
    let imported = fonts::upload(&chromium, &project).await;
    let mut ops = fixtures::repo_wrap();
    ops.extend(fixtures::typography());
    ops.extend(fixtures::missing_control());
    ops.extend(fixtures::long_page());
    let created = chromium.batch(&project, imported, json!(ops)).await;
    let revision = imported + 1;
    assert_eq!(created["committedRevision"], revision, "{created}");
    let snapshot = chromium
        .get(&format!("/api/v1/projects/{project}/snapshot"))
        .await;
    assert_eq!(
        snapshot,
        native
            .get(&format!("/api/v1/projects/{project}/snapshot"))
            .await,
        "one identical authored store snapshot"
    );
    let mut comparison = Comparison::default();
    for frame in ["frame", "typography", "long-page"] {
        let expected = descendants(&ops, frame);
        let mut at_widths = Vec::new();
        for width in [1040, 721, 720, 390] {
            for state in ["default", "hover"] {
                // Fully settle each exact job before asking for the next viewport/state.
                let browser = chromium
                    .scene(&project, revision, frame, width, state)
                    .await;
                let rust = native.scene(&project, revision, frame, width, state).await;
                metadata(&browser, &rust);
                if frame == "long-page" {
                    for job in [&browser, &rust] {
                        let h = job["scene"]["provenance"]["viewport"]["height"].as_f64();
                        let png = h.map(|h| (h * 0.25).ceil());
                        assert_eq!(job["image"]["height"].as_f64(), png, "hug PNG height");
                    }
                }
                compare::no_missing(&browser);
                compare::no_missing(&rust);
                let label = format!("fixture={frame}/viewport={width}x1200/state={state}");
                comparison.scenes(&label, &browser["scene"], &rust["scene"], &expected);
                if state == "default" {
                    at_widths.push(browser["scene"]["nodes"].clone());
                }
                if frame == "typography" && state == "hover" {
                    assert_eq!(rust["scene"]["nodes"][0]["fill"], "#edf5ff");
                }
            }
        }
        let descendants_changed = at_widths[0]
            .as_array()
            .unwrap()
            .iter()
            .zip(at_widths[3].as_array().unwrap())
            .skip(1)
            .any(|(a, b)| a["bounds"] != b["bounds"]);
        assert!(descendants_changed, "descendants actually reflow: {frame}");
        if frame == "long-page" {
            let height = |nodes: &Value| nodes[0]["bounds"]["height"].as_f64().unwrap();
            assert!(
                height(&at_widths[3]) > height(&at_widths[0]),
                "hug root grows"
            );
        }
        if frame == "typography" {
            for (index, expected_x) in [(1, 24.0), (2, 12.0)] {
                let cards = at_widths[index]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|n| n["id"] == "type-cards")
                    .unwrap();
                assert_eq!(
                    cards["bounds"]["x"].as_f64(),
                    Some(expected_x),
                    "721/720 full breakpoint rule"
                );
            }
        }
    }
    // Not an eligibility label: actual diagnostics must prove these scalars are missing.
    for server in [&chromium, &native] {
        let job = server
            .scene(&project, revision, "missing-control", 720, "default")
            .await;
        let issues = job["diagnostics"].as_array().unwrap();
        let missing = issues
            .iter()
            .filter_map(Value::as_str)
            .find(|d| d.starts_with("MISSING_GLYPH node=missing-text "))
            .expect("actual missing diagnostic");
        for scalar in ["U+2318", "U+1F984", "U+4E00"] {
            assert!(missing.contains(scalar), "{missing}");
        }
        eprintln!("EXCLUDED actual-missing-only backend={:?} fixture=missing-control viewport=720x1200 node=missing-text text=Missing ⌘ 🦄 一 diagnostic={missing}", server.state.config.render_backend);
    }
    comparison.finish();
    runtime::unchanged(&checked_sources);
}
