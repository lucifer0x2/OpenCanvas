//! Unsupported native resources fail explicitly, without browser fallback or fake readiness.
#[allow(dead_code)]
mod native_support;
use base64::{engine::general_purpose::STANDARD, Engine};
use native_support::{design, Server};
use serde_json::{json, Value};

#[tokio::test]
async fn color_table_metrics_and_svg_do_not_require_native_color_glyph_paint() {
    let s = Server::start().await;
    let p = s.project().await;
    let mut bytes = include_bytes!("../../../renderer/fonts/NotoSans.ttf").to_vec();
    let count = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    let entry = bytes[12..12 + count * 16]
        .as_chunks_mut::<16>()
        .0
        .iter_mut()
        .find(|entry| &entry[..4] == b"name")
        .unwrap();
    entry[..4].copy_from_slice(b"COLR");
    let imported: Value = s
        .client
        .post(format!("{}/api/v1/projects/{p}/fonts", s.url))
        .header("origin", &s.url)
        .json(&json!({"expectedRevision":0,"idempotencyKey":"color-font",
            "mediaType":"font/ttf","base64":STANDARD.encode(&bytes),"font":{"id":"color",
                "family":"Unsupported Color","weight":400,"style":"normal"}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(imported["committedRevision"], 1, "{imported}");
    let mut ops = design();
    ops[2]["props"]["fontId"] = json!("color");
    assert_eq!(s.batch(&p, 1, ops).await["committedRevision"], 2);
    let job = s.render(&p, 2, "hover").await;
    assert_eq!(job["status"], "ready", "{job}");
    assert_eq!(job["error"]["code"], "UNSUPPORTED_FEATURE", "{job}");
    assert!(job.get("imageUrl").is_none());
    assert!(job["readiness"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == "color"
            && r["status"] == "failed"
            && r["code"] == "UNSUPPORTED_FEATURE"));
    assert_eq!(job["provenance"]["engine"], "native");
    for settings in [
        json!([{"scale":1}]),
        json!([{"format":"svg"},{"scale":1}]),
        json!([{"scale":1},{"format":"svg"}]),
    ] {
        let result = s
            .call(
                "export_nodes",
                json!({"projectId":p,"nodeIds":["text"],"settings":settings}),
            )
            .await;
        assert_eq!(
            result["exports"][0]["error"]["code"], "UNSUPPORTED_FEATURE",
            "{result}"
        );
        assert!(result["exports"][0].get("files").is_none());
    }
    // This table-renaming control tests the structural gate only, not a valid
    // COLRv1 fixture. The real-font cross-backend test lives in svg_color_font.rs.
    let svg = s
        .call(
            "export_nodes",
            json!({"projectId":p,"nodeIds":["text"],
        "settings":[{"format":"svg"}]}),
        )
        .await;
    assert_eq!(svg["exports"][0]["status"], "ready", "{svg}");
    let markup = s
        .client
        .get(svg["exports"][0]["files"][0]["url"].as_str().unwrap())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(markup.contains(&STANDARD.encode(&bytes)));
    assert!(!s.root.path().join("worker").exists());
    assert!(!job.to_string().contains(s.root.path().to_str().unwrap()));
}
