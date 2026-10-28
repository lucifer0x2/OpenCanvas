//! B05: import_asset answers with apply_batch's top-level commit fields, and image `fit`
//! keeps one vocabulary from authored props through resolvedProps to a rendered scene.
mod support;
use opencanvas_storage::IMAGE_FITS;
use serde_json::{json, Value};
use support::Server;

const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=";

async fn import(s: &Server, p: &str, key: &str) -> Value {
    let args = json!({"projectId":p,"expectedRevision":0,"idempotencyKey":key,"mediaType":"image/png","base64":PNG});
    s.call("import_asset", args).await
}

#[tokio::test]
async fn import_asset_puts_commit_fields_at_the_top_like_apply_batch() {
    let s = Server::start().await;
    let p = s.project().await;
    let first = import(&s, &p, "image").await;
    assert!(first.get("error").is_none(), "{first}");
    for key in [
        "projectId",
        "committedRevision",
        "changedIds",
        "renderStatus",
        "replayed",
    ] {
        assert_eq!(first[key], first["result"][key], "{key} in {first}");
    }
    assert_eq!(first["committedRevision"], 1);
    assert_eq!(first["replayed"], false);
    assert!(first["asset"]["id"].is_string(), "{first}");
    let replay = import(&s, &p, "image").await;
    assert_eq!(replay["committedRevision"], 1, "{replay}");
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["result"]["replayed"], true);
    // The top-level revision is the one the next mutation expects.
    let revision = first["committedRevision"].as_u64().unwrap();
    let page = json!([{"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}}]);
    assert_eq!(
        s.batch(&p, revision, "next", page).await["committedRevision"],
        2
    );
}

#[tokio::test]
async fn every_fit_reaches_the_rendered_scene_under_its_authored_name() {
    let s = Server::start().await;
    let p = s.project().await;
    let asset = import(&s, &p, "image").await["asset"]["id"].clone();
    let mut ops = vec![
        json!({"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}}),
        json!({"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":200,"height":60,"fill":"#ffffff","layout":{"mode":"row","gap":10}}}),
    ];
    for fit in IMAGE_FITS {
        ops.push(json!({"op":"create_node","id":fit,"kind":"image","parentId":"frame","props":{"assetId":asset,"fit":fit,"width":40,"height":20}}));
    }
    assert_eq!(
        s.batch(&p, 1, "images", json!(ops)).await["committedRevision"],
        2
    );

    let context = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"frame"}),
        )
        .await;
    let nodes = context["nodes"].as_array().unwrap();
    for fit in IMAGE_FITS {
        let node = nodes.iter().find(|n| n["id"] == fit).unwrap();
        assert_eq!(node["resolvedProps"]["fit"], fit, "{node}");
    }

    let args =
        json!({"projectId":p,"revision":2,"frameId":"frame","viewport":{"width":200,"height":60}});
    let job = s.call("render_frame", args).await;
    let job = s.wait_job(job["jobId"].as_str().unwrap()).await;
    assert_eq!(job["status"], "ready", "{job}");
    let scene = job["scene"]["nodes"].as_array().unwrap();
    for fit in IMAGE_FITS {
        let node = scene.iter().find(|n| n["id"] == fit).unwrap();
        assert_eq!(node["fit"], fit, "{node}");
    }

    let bad = json!([{"op":"update_node","id":"cover","props":{"fit":"fill"}}]);
    let error = s.batch(&p, 2, "fill", bad).await;
    assert_eq!(error["error"]["code"], "VALIDATION_FAILED", "{error}");
    assert!(
        error.to_string().contains("Use \\\"fit\\\" to stretch"),
        "{error}"
    );
}
