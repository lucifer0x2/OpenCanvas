//! `export_nodes` against real Chromium: node crops at 1x/2x/3x and target widths, transparent
//! backgrounds, ancestor isolation, per-node errors and the file download route.
mod export_support;
#[allow(dead_code)]
mod support;
use export_support::{decode, design, pixel};
use serde_json::{json, Value};
use support::Server;

async fn export(s: &Server, p: &str, extra: Value) -> Value {
    let mut args = json!({"projectId":p});
    args.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    s.call("export_nodes", args).await
}

async fn download(s: &Server, file: &Value) -> (u32, u32, Vec<u8>) {
    let response = s
        .client
        .get(file["url"].as_str().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "image/png");
    let disposition = response.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .to_owned();
    assert!(
        disposition.contains(file["fileName"].as_str().unwrap()),
        "{disposition}"
    );
    let bytes = response.bytes().await.unwrap();
    assert_eq!(bytes.len() as u64, file["bytes"].as_u64().unwrap());
    decode(&bytes)
}

fn files(item: &Value) -> Vec<(String, u64, u64)> {
    assert_eq!(item["status"], "ready", "{item}");
    item["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["fileName"].as_str().unwrap().to_owned(),
                f["width"].as_u64().unwrap(),
                f["height"].as_u64().unwrap(),
            )
        })
        .collect()
}

#[tokio::test]
async fn exports_node_crops_with_transparent_background() {
    let s = Server::start().await;
    let p = s.project().await;
    let made = s.batch(&p, 0, "design", design()).await;
    assert_eq!(made["committedRevision"], 1, "{made}");

    // Cold worker: an unwaited export is still pending and its file is not served yet.
    let cold = export(
        &s,
        &p,
        json!({"nodeIds":["frame"],"settings":[{"scale":0.5}],"waitMs":0}),
    )
    .await;
    let cold_job = cold["exports"][0]["jobId"].as_str().unwrap().to_owned();
    if cold["exports"][0]["status"] == "pending" {
        let early = s
            .client
            .get(format!("{}/api/v1/jobs/{cold_job}/files/0", s.url))
            .send()
            .await
            .unwrap();
        assert_eq!(early.status(), 409);
    }

    // AC3, AC9, AC10: one call, 2x crop, transparent corner, same job on repeat.
    let args = json!({"nodeIds":["box"],"settings":[{"scale":2}]});
    let first = export(&s, &p, args.clone()).await;
    let item = &first["exports"][0];
    assert_eq!(files(item), [("Box@2x.png".to_owned(), 80, 80)]);
    let image = download(&s, &item["files"][0]).await;
    assert_eq!((image.0, image.1), (80, 80));
    assert_eq!(pixel(&image, 0, 0)[3], 0);
    assert_eq!(pixel(&image, 40, 40), [239, 68, 68, 255]);
    let again = export(&s, &p, args).await;
    assert_eq!(again["exports"][0]["jobId"], item["jobId"]);
    let job = item["jobId"].as_str().unwrap();
    let missing = s
        .client
        .get(format!("{}/api/v1/jobs/{job}/files/9", s.url))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
    assert_eq!(
        missing.json::<Value>().await.unwrap()["error"]["code"],
        "FILE_NOT_FOUND"
    );

    // AC4: fractional width rounds up per scale; text keeps a transparent background.
    let label = export(
        &s,
        &p,
        json!({"nodeIds":["label"],"settings":[{"scale":3}]}),
    )
    .await;
    let label = &label["exports"][0];
    assert_eq!(files(label)[0].1, 401);
    let image = download(&s, &label["files"][0]).await;
    let clear = image.2.chunks(4).filter(|px| px[3] == 0).count();
    assert!(
        clear * 2 >= (image.0 * image.1) as usize,
        "{clear} clear pixels"
    );

    // AC6, AC7: ancestor clip/opacity do not apply; the root frame exports at authored size.
    let both = export(&s, &p, json!({"nodeIds":["inner","frame"]})).await;
    assert_eq!(
        files(&both["exports"][0]),
        [("inner.png".to_owned(), 60, 60)]
    );
    let inner = download(&s, &both["exports"][0]["files"][0]).await;
    assert_eq!(pixel(&inner, 45, 30)[3], 255);
    assert_eq!(
        files(&both["exports"][1]),
        [("Screen.png".to_owned(), 320, 200)]
    );

    // AC8: errors stay on their own item; the healthy node still exports.
    let mixed = export(&s, &p, json!({"nodeIds":["page","nope","thin","box"]})).await;
    let codes: Vec<_> = mixed["exports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["error"]["code"].clone())
        .collect();
    assert_eq!(
        codes[..3],
        [
            json!("EXPORT_UNSUPPORTED"),
            json!("NODE_NOT_FOUND"),
            json!("EMPTY_BOUNDS")
        ],
        "{mixed}"
    );
    assert_eq!(mixed["exports"][3]["status"], "ready");
    let budget = export(&s, &p, json!({"nodeIds":["huge"],"settings":[{"scale":4}]})).await;
    assert_eq!(
        budget["exports"][0]["error"]["code"], "PIXEL_BUDGET",
        "{budget}"
    );
    let range = export(
        &s,
        &p,
        json!({"nodeIds":["box"],"settings":[{"width":2000}]}),
    )
    .await;
    assert_eq!(
        range["exports"][0]["error"]["code"], "EXPORT_SCALE_RANGE",
        "{range}"
    );

    // AC5, AC11: the node's own exportSettings, a target width, and a compact response.
    let marked = json!([{"op":"update_node","id":"box","props":{"exportSettings":[{"scale":1},{"scale":3},{"width":96,"suffix":"-96"}]}}]);
    assert_eq!(s.batch(&p, 1, "mark", marked).await["committedRevision"], 2);
    let stale = export(&s, &p, json!({"nodeIds":["box"],"revision":1})).await;
    assert_eq!(stale["error"]["code"], "REVISION_UNAVAILABLE");
    let own = export(&s, &p, json!({"nodeIds":["box"],"revision":2})).await;
    let item = &own["exports"][0];
    let expected = [
        ("Box.png", 40, 40),
        ("Box@3x.png", 120, 120),
        ("Box-96.png", 96, 96),
    ];
    let expected: Vec<_> = expected
        .iter()
        .map(|(n, w, h)| (n.to_string(), *w, *h))
        .collect();
    assert_eq!(files(item), expected);
    assert_eq!(item["files"][2]["scale"], 2.4);
    let wide = download(&s, &item["files"][2]).await;
    assert_eq!((wide.0, wide.1), (96, 96));
    let size = serde_json::to_vec(&own).unwrap().len();
    assert!(size <= 1536, "{size} bytes");
    let text = own.to_string();
    assert!(!text.contains("\"scene\"") && !text.contains("base64"));
    let unmarked = export(&s, &p, json!({"nodeIds":["label"]})).await;
    assert_eq!(
        files(&unmarked["exports"][0]),
        [("label.png".to_owned(), 134, 28)]
    );

    // The job record serves the same files without a scene.
    let record = s
        .get(&format!("/api/v1/jobs/{}", item["jobId"].as_str().unwrap()))
        .await;
    assert_eq!(record["kind"], "export");
    assert_eq!(record["files"], item["files"]);
    assert!(record.get("scene").is_none());
    let _ = s.wait_job(&cold_job).await;
}

#[tokio::test]
async fn invalid_settings_argument_fails_the_whole_call() {
    let s = Server::start().await;
    let p = s.project().await;
    s.batch(&p, 0, "design", design()).await;
    let dup = export(
        &s,
        &p,
        json!({"nodeIds":["box"],"settings":[{"scale":2},{"scale":2}]}),
    )
    .await;
    assert_eq!(dup["error"]["code"], "VALIDATION_FAILED");
    assert_eq!(dup["error"]["field"], "settings[1]");
    let mixed = export(
        &s,
        &p,
        json!({"nodeIds":["box"],"settings":[{"scale":2,"width":10}]}),
    )
    .await;
    assert_eq!(mixed["error"]["field"], "settings[0]");
}
