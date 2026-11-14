//! `import_asset` with `sourceProjectId` copies an asset, or a font together with its
//! registration, from another project. The copy is an ordinary content-addressed import:
//! same asset ID, same replay, and the font renders at once in the target project.
#[allow(dead_code)]
mod support;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use support::Server;

const LATO: &[u8] = include_bytes!("../../../fixtures/fonts/Lato-Regular.ttf");
const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=";

async fn create(s: &Server, slug: &str) -> String {
    let created = s
        .call("create_project", json!({"slug":slug,"name":slug}))
        .await;
    created["projectId"].as_str().unwrap().into()
}
fn import(project: &str, revision: u64, key: &str, source: Value) -> Value {
    let mut args = json!({"projectId":project,"expectedRevision":revision,"idempotencyKey":key});
    let fields = source.as_object().unwrap().clone();
    args.as_object_mut().unwrap().extend(fields);
    args
}
/// A source project holding one registered font and one image.
async fn source(s: &Server) -> (String, Value, Value) {
    let p = create(s, "brand").await;
    let font = json!({"mediaType":"font/ttf","base64":STANDARD.encode(LATO),
        "font":{"id":"lato","family":"Lato","weight":400,"style":"normal","license":"OFL-1.1"}});
    let font = s.call("import_asset", import(&p, 0, "font", font)).await;
    assert_eq!(font["committedRevision"], 1, "{font}");
    let png = json!({"mediaType":"image/png","base64":PNG});
    let png = s.call("import_asset", import(&p, 1, "png", png)).await;
    assert_eq!(png["committedRevision"], 2, "{png}");
    (p, font["asset"].clone(), png["asset"].clone())
}
async fn bytes(s: &Server, url: &str) -> Vec<u8> {
    let response = s.client.get(url).send().await.unwrap();
    assert_eq!(response.status(), 200, "{url}");
    response.bytes().await.unwrap().to_vec()
}

#[tokio::test]
async fn copies_fonts_with_registration_and_assets_by_id() {
    let s = Server::start().await;
    let (_, font_asset, png_asset) = source(&s).await;
    let target = create(&s, "site").await;
    // The source may be named by slug, like every other project argument.
    let item = json!({"sourceProjectId":"brand","fontId":"lato"});
    let font = s
        .call("import_asset", import(&target, 0, "lato", item.clone()))
        .await;
    assert_eq!(font["committedRevision"], 1, "{font}");
    assert_eq!(font["asset"], font_asset);
    assert_eq!(font["changedIds"], json!([font_asset["id"], "lato"]));
    let replay = s
        .call("import_asset", import(&target, 0, "lato", item))
        .await;
    assert_eq!(replay["replayed"], true, "{replay}");
    assert_eq!(replay["asset"], font_asset);
    let item = json!({"sourceProjectId":"brand","assetId":png_asset["id"]});
    let png = s
        .call("import_asset", import(&target, 1, "png", item))
        .await;
    assert_eq!(png["committedRevision"], 2, "{png}");
    assert_eq!(png["asset"], png_asset);
    assert_eq!(png["changedIds"], json!([png_asset["id"]]));

    let fonts = s.get(&format!("/api/v1/projects/{target}/fonts")).await;
    let lato = fonts["fonts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "lato")
        .unwrap_or_else(|| panic!("{fonts}"));
    assert_eq!(lato["family"], "Lato");
    assert_eq!(lato["license"], "OFL-1.1");
    assert_eq!(lato["source"], "project");
    assert_eq!(bytes(&s, lato["downloadUrl"].as_str().unwrap()).await, LATO);
    let asset = format!(
        "{}/api/v1/projects/{target}/assets/{}",
        s.url,
        png_asset["id"].as_str().unwrap()
    );
    assert_eq!(bytes(&s, &asset).await, STANDARD.decode(PNG).unwrap());

    // The copied registration is usable at once: text set in it renders.
    let mut design = support::design();
    design[2]["props"]["fontId"] = json!("lato");
    let created = s.batch(&target, 2, "design", design).await;
    assert_eq!(created["committedRevision"], 3, "{created}");
    let render = s.call("render_frame", json!({"projectId":target,"revision":3,"frameId":"frame","viewport":{"width":320,"height":200}})).await;
    let job = match render["status"].as_str() {
        Some("pending") => s.wait_job(render["jobId"].as_str().unwrap()).await,
        _ => render,
    };
    assert_eq!(job["status"], "ready", "{job}");
}

#[tokio::test]
async fn copy_rejects_missing_items_the_bundled_font_and_mixed_sources() {
    let s = Server::start().await;
    let (_, _, png_asset) = source(&s).await;
    let target = create(&s, "site").await;
    for (item, code) in [
        (
            json!({"sourceProjectId":"nope","fontId":"lato"}),
            "PROJECT_NOT_FOUND",
        ),
        (
            json!({"sourceProjectId":"brand","assetId":"asset_missing"}),
            "ASSET_NOT_FOUND",
        ),
        (
            json!({"sourceProjectId":"brand","fontId":"missing"}),
            "FONT_NOT_FOUND",
        ),
        (
            json!({"sourceProjectId":"brand","fontId":"font_bundled"}),
            "INVALID_REQUEST",
        ),
        (
            json!({"sourceProjectId":"brand","assetId":png_asset["id"],"mediaType":"image/png"}),
            "INVALID_REQUEST",
        ),
        (
            json!({"sourceProjectId":"brand","assetId":png_asset["id"],"fontId":"lato"}),
            "INVALID_REQUEST",
        ),
    ] {
        let result = s
            .call("import_asset", import(&target, 0, "bad", item.clone()))
            .await;
        assert_eq!(result["error"]["code"], code, "{item}: {result}");
    }
    let snapshot = s.get(&format!("/api/v1/projects/{target}/snapshot")).await;
    assert_eq!(snapshot["revision"], 0);
}
