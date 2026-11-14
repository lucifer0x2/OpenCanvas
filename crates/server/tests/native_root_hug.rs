//! A hug-height root frame renders, inspects, reads and exports at its computed height.
#[allow(dead_code)]
mod export_support;
#[allow(dead_code)]
mod native_support;
use export_support::decode;
use native_support::Server;
use serde_json::{json, Value};

/// Row of a 40 and a 60 px box with 16 px padding; a column at 250 px and narrower.
fn design() -> Value {
    let rect = |id: &str, h: u32| json!({"op":"create_node","id":id,"kind":"rectangle","parentId":"root","props":{"width":100,"height":h,"fill":"#ef4444"}});
    json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"root","kind":"frame","parentId":"page","props":{"width":320,"height":"hug","fill":"#ffffff",
            "layout":{"mode":"row","padding":16,"gap":8,"breakpoints":[{"maxWidth":250,"layout":{"mode":"column","padding":16,"gap":8}}]}}},
        rect("a", 40),
        rect("b", 60),
    ])
}

async fn bytes(s: &Server, url: &Value) -> Vec<u8> {
    let response = s.client.get(url.as_str().unwrap()).send().await.unwrap();
    response.bytes().await.unwrap().to_vec()
}

async fn render(s: &Server, p: &str, width: u32, height: u32) -> Value {
    let args = json!({"projectId":p,"revision":1,"frameId":"root","viewport":{"width":width,"height":height}});
    let pending = s.call("render_frame", args).await;
    s.wait(pending["jobId"].as_str().unwrap()).await
}

#[tokio::test]
async fn render_inspect_context_and_export_use_the_computed_height() {
    let s = Server::start().await;
    let p = s.project().await;
    assert_eq!(s.batch(&p, 0, design()).await["committedRevision"], 1);

    let job = render(&s, &p, 320, 900).await;
    assert_eq!(job["status"], "ready", "{job}");
    let viewport = &job["scene"]["provenance"]["viewport"];
    assert_eq!(
        (viewport["width"].as_f64(), viewport["height"].as_f64()),
        (Some(320.), Some(92.))
    );
    let png = decode(&bytes(&s, &job["imageUrl"]).await);
    assert_eq!((png.0, png.1), (320, 92));
    // The requested height is ignored, so another one reuses the same scene job.
    assert_eq!(render(&s, &p, 320, 600).await["sceneKey"], job["sceneKey"]);

    let inspect = s
        .call(
            "inspect_layout",
            json!({"projectId":p,"jobId":job["jobId"],"sceneKey":job["sceneKey"]}),
        )
        .await;
    let root = &inspect["nodes"][0];
    assert_eq!(root["id"], "root", "{inspect}");
    assert_eq!(root["bounds"]["height"], 92.);

    let narrow = render(&s, &p, 200, 900).await;
    assert_eq!(narrow["scene"]["provenance"]["viewport"]["height"], 140.);
    let args = json!({"projectId":p,"frameId":"root","depth":0,
        "viewports":[{"width":320,"height":900},{"width":200,"height":900}]});
    let mut context = s.call("get_design_context", args.clone()).await;
    for _ in 0..200 {
        if context["computed"]["status"] != "pending" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        context = s.call("get_design_context", args.clone()).await;
    }
    assert_eq!(context["computed"]["status"], "ready", "{context}");
    let bounds = &context["nodes"][0]["computedBounds"];
    assert_eq!(
        (bounds[0]["height"].as_f64(), bounds[1]["height"].as_f64()),
        (Some(92.), Some(140.))
    );

    let settings = json!([{"scale":1},{"format":"svg"}]);
    let exports = s
        .call(
            "export_nodes",
            json!({"projectId":p,"nodeIds":["root","b"],"settings":settings}),
        )
        .await;
    let root = &exports["exports"][0];
    assert_eq!(root["status"], "ready", "{exports}");
    let png = decode(&bytes(&s, &root["files"][0]["url"]).await);
    assert_eq!((png.0, png.1), (320, 92));
    assert_eq!(root["files"][1]["height"], 92.);
    assert_eq!(exports["exports"][1]["status"], "ready", "{exports}");
}
