//! `export_nodes` with `format:"svg"`: a self-contained file built from the scene, served as
//! `image/svg+xml`, with original image bytes embedded and strict failure on a missing asset.
#[allow(dead_code)]
mod export_support;
#[allow(dead_code)]
mod support;
use base64::Engine;
use export_support::design;
use serde_json::{json, Value};
use support::Server;

const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=";

async fn export(s: &Server, p: &str, node: &str, settings: Value) -> Value {
    let args = json!({"projectId":p,"nodeIds":[node],"settings":settings});
    s.call("export_nodes", args).await["exports"][0].clone()
}

async fn download(s: &Server, file: &Value) -> String {
    let response = s
        .client
        .get(file["url"].as_str().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let headers = response.headers();
    assert_eq!(headers["content-type"], "image/svg+xml");
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert!(headers["content-disposition"]
        .to_str()
        .unwrap()
        .starts_with("attachment"));
    response.text().await.unwrap()
}

/// The design plus one image node over the imported PNG, committed at revision 2.
async fn with_image(s: &Server, p: &str) -> String {
    let imported = s.call("import_asset", json!({"projectId":p,"expectedRevision":0,"idempotencyKey":"image","mediaType":"image/png","base64":PNG})).await;
    assert!(imported.get("error").is_none(), "{imported}");
    let snapshot = s.get(&format!("/api/v1/projects/{p}/snapshot")).await;
    let asset = snapshot["assets"][0]["id"].as_str().unwrap().to_owned();
    let mut ops = design();
    ops.as_array_mut().unwrap().push(json!({"op":"create_node","id":"image","kind":"image","parentId":"frame","props":{"name":"Photo","assetId":asset,"x":250,"y":100,"width":32,"height":32,"radius":4,"fit":"cover"}}));
    assert_eq!(s.batch(p, 1, "design", ops).await["committedRevision"], 2);
    asset
}

#[tokio::test]
async fn exports_self_contained_svg_beside_png() {
    let s = Server::start().await;
    let p = s.project().await;
    with_image(&s, &p).await;

    // AC6: mime, name, original bytes embedded, nothing pointing back at the server.
    let item = export(&s, &p, "image", json!([{"format":"svg"},{"scale":2}])).await;
    assert_eq!(item["status"], "ready", "{item}");
    let svg = &item["files"][0];
    assert_eq!(svg["mimeType"], "image/svg+xml");
    assert_eq!(svg["fileName"], "Photo.svg");
    assert_eq!(
        (svg["width"].as_f64(), svg["height"].as_f64()),
        (Some(32.0), Some(32.0))
    );
    assert!(
        svg.get("diagnostics").is_none(),
        "AC8: no text, no diagnostic"
    );
    assert_eq!(item["files"][1]["fileName"], "Photo@2x.png");
    let markup = download(&s, svg).await;
    assert_eq!(markup.len() as u64, svg["bytes"].as_u64().unwrap());
    let doc = roxmltree::Document::parse(&markup).unwrap();
    let href = doc
        .descendants()
        .find(|n| n.has_tag_name("image"))
        .and_then(|n| n.attribute("href"))
        .unwrap();
    let data = href.strip_prefix("data:image/png;base64,").unwrap();
    let engine = base64::engine::general_purpose::STANDARD;
    assert_eq!(engine.decode(data).unwrap(), engine.decode(PNG).unwrap());
    assert!(!markup.contains("127.0.0.1") && !markup.contains("/api/v1/"));

    // AC8: text flags that it is not outlined; svgIdAttribute marks nodes.
    let label = export(
        &s,
        &p,
        "label",
        json!([{"format":"svg","svgIdAttribute":true}]),
    )
    .await;
    let file = &label["files"][0];
    assert_eq!(
        file["diagnostics"],
        json!(["SVG_TEXT_NOT_OUTLINED"]),
        "{label}"
    );
    let markup = download(&s, file).await;
    assert!(markup.contains("id=\"label\"") && markup.contains("Export me"));

    // A scale on an SVG setting fails the whole call on its own field.
    let scaled = s
        .call(
            "export_nodes",
            json!({"projectId":p,"nodeIds":["box"],"settings":[{"format":"svg","scale":2}]}),
        )
        .await;
    assert_eq!(scaled["error"]["code"], "VALIDATION_FAILED", "{scaled}");
}

#[tokio::test]
async fn missing_asset_bytes_fail_the_svg_instead_of_dropping_the_image() {
    let s = Server::start().await;
    let p = s.project().await;
    with_image(&s, &p).await;
    let files: Vec<_> = std::fs::read_dir(s.root.path().join("test-project/assets"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .collect();
    assert_eq!(files.len(), 1);
    std::fs::remove_file(&files[0]).unwrap();
    // AC10: no partial SVG without its image.
    let item = export(&s, &p, "image", json!([{"format":"svg"}])).await;
    assert_eq!(item["error"]["code"], "RESOURCE_NOT_READY", "{item}");
    assert!(item
        .get("files")
        .is_none_or(|f| f.as_array().unwrap().is_empty()));
}
