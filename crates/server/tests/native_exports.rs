//! Same export contract as Chromium, with absent Node and layout worker paths.
mod export_support;
#[allow(dead_code)]
mod native_support;
use export_support::{decode, design, pixel};
use native_support::Server;
use serde_json::{json, Value};

async fn export(s: &Server, p: &str, nodes: Value, settings: Value) -> Value {
    s.call(
        "export_nodes",
        json!({"projectId":p,"nodeIds":nodes,"settings":settings}),
    )
    .await
}

async fn download(s: &Server, file: &Value) -> Vec<u8> {
    let response = s
        .client
        .get(file["url"].as_str().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers()["content-type"],
        file["mimeType"].as_str().unwrap()
    );
    assert!(response.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .contains(file["fileName"].as_str().unwrap()));
    let bytes = response.bytes().await.unwrap().to_vec();
    assert_eq!(bytes.len() as u64, file["bytes"].as_u64().unwrap());
    bytes
}

#[tokio::test]
async fn native_exports_share_crops_scales_targets_transparency_svg_and_errors() {
    let s = Server::start().await;
    let p = s.project().await;
    assert_eq!(s.batch(&p, 0, design()).await["committedRevision"], 1);
    let settings = json!([{"scale":1},{"scale":2},{"scale":2.4},{"width":96,"suffix":"-w"},{"height":96,"suffix":"-h"},{"format":"svg"}]);
    let result = export(&s, &p, json!(["box"]), settings.clone()).await;
    let item = &result["exports"][0];
    assert_eq!(item["status"], "ready", "{result}");
    for (file, size) in item["files"].as_array().unwrap()[..5]
        .iter()
        .zip([40, 80, 96, 96, 96])
    {
        let image = decode(&download(&s, file).await);
        assert_eq!((image.0, image.1), (size, size));
        assert_eq!(pixel(&image, 0, 0)[3], 0);
        assert_eq!(pixel(&image, size / 2, size / 2), [239, 68, 68, 255]);
    }
    assert_eq!(item["files"][3]["scale"], 2.4);
    let svg = String::from_utf8(download(&s, &item["files"][5]).await).unwrap();
    assert!(svg.contains("<svg") && svg.contains("#ef4444"));
    let again = export(&s, &p, json!(["box"]), settings).await;
    assert_eq!(again["exports"][0]["jobId"], item["jobId"]);
    let record = s
        .get(&format!("/api/v1/jobs/{}", item["jobId"].as_str().unwrap()))
        .await;
    assert!(record.get("scene").is_none());
    assert_eq!(record["provenance"]["sourceSceneKey"], record["sceneKey"]);
    assert_eq!(record["provenance"]["engine"], "native");
    assert!(record["provenance"].get("browserVersion").is_none());
    let targets = export(
        &s,
        &p,
        json!(["frame"]),
        json!([{"width":96,"suffix":"-w"},{"height":96,"suffix":"-h"}]),
    )
    .await;
    for (file, size) in targets["exports"][0]["files"]
        .as_array()
        .unwrap()
        .iter()
        .zip([(96, 60), (154, 96)])
    {
        let image = decode(&download(&s, file).await);
        assert_eq!((image.0, image.1), size);
    }

    let fractional = export(&s, &p, json!(["label"]), json!([{"scale":3}])).await;
    let text_image = decode(&download(&s, &fractional["exports"][0]["files"][0]).await);
    assert_eq!((text_image.0, text_image.1), (401, 84));
    assert!(
        text_image
            .2
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|px| px[3] == 0)
            .count()
            > (401 * 84 / 2)
    );
    let isolated = export(&s, &p, json!(["inner"]), json!([{"scale":1}])).await;
    let isolated = decode(&download(&s, &isolated["exports"][0]["files"][0]).await);
    assert_eq!(pixel(&isolated, 45, 30), [37, 99, 235, 255]);
    let svg_only = export(&s, &p, json!(["label"]), json!([{"format":"svg"}])).await;
    let markup =
        String::from_utf8(download(&s, &svg_only["exports"][0]["files"][0]).await).unwrap();
    assert!(markup.contains("@font-face") && markup.contains("data:font/ttf;base64,"));
    assert!(!markup.contains("127.0.0.1"));

    let mixed = export(
        &s,
        &p,
        json!(["page", "missing", "thin", "box"]),
        json!([{"scale":1}]),
    )
    .await;
    for (item, code) in mixed["exports"].as_array().unwrap().iter().zip([
        "EXPORT_UNSUPPORTED",
        "NODE_NOT_FOUND",
        "EMPTY_BOUNDS",
    ]) {
        assert_eq!(item["error"]["code"], code, "{mixed}");
    }
    assert_eq!(mixed["exports"][3]["status"], "ready");
    let budget = export(&s, &p, json!(["huge"]), json!([{"scale":4}])).await;
    assert_eq!(budget["exports"][0]["error"]["code"], "PIXEL_BUDGET");
    let range = export(&s, &p, json!(["box"]), json!([{"width":2000}])).await;
    assert_eq!(range["exports"][0]["error"]["code"], "EXPORT_SCALE_RANGE");
    let invalid = export(&s, &p, json!(["box"]), json!([{"scale":0}])).await;
    assert_eq!(invalid["error"]["code"], "INVALID_REQUEST");
    let marked = json!([{"op":"update_node","id":"box","props":{"exportSettings":[{"scale":1},{"width":96,"suffix":"-own"}]}}]);
    assert_eq!(s.batch(&p, 1, marked).await["committedRevision"], 2);
    let own = s
        .call("export_nodes", json!({"projectId":p,"nodeIds":["box"]}))
        .await;
    assert_eq!(own["exports"][0]["files"][1]["fileName"], "Box-own.png");
    assert_eq!(own["exports"][0]["files"][1]["width"], 96);
    assert!(!s.root.path().join("worker").exists());
}
