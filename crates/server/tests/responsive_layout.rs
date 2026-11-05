#[allow(dead_code)]
mod multi_support;
mod support;
use multi_support::settled;
use serde_json::{json, Value};
use support::Server;

fn fixture() -> Value {
    json!([
        {"op":"upsert_token","token":{"id":"padding","name":"Layout padding","type":"dimension",
            "value":{"default":16,"themes":{"dense":20}}}},
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{
            "width":1200,"height":400,"layout":{"mode":"free","padding":32,"breakpoints":[
                {"maxWidth":600,"layout":{"mode":"column","padding":{"tokenId":"padding"},"gap":12}}
            ]}
        }},
        {"op":"create_node","id":"panel","kind":"group","parentId":"frame","props":{
            "x":40,"y":30,"width":"50%","minWidth":220,"height":80
        }},
        {"op":"create_node","id":"detail","kind":"rectangle","parentId":"panel","props":{
            "x":10,"y":6,"width":"50%","maxWidth":125,"height":"25%","fill":"#336699"
        }},
        {"op":"create_node","id":"aside","kind":"rectangle","parentId":"frame","props":{
            "x":440,"y":30,"width":100,"height":40,"fill":"#225588"
        }},
        {"op":"create_node","id":"fixed","kind":"frame","parentId":"page","props":{"width":1200,"height":400}},
        {"op":"create_node","id":"fixed-box","kind":"rectangle","parentId":"fixed","props":{
            "x":40,"y":30,"width":100,"height":40
        }}
    ])
}

fn node<'a>(response: &'a Value, id: &str) -> &'a Value {
    response["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == id)
        .unwrap()
}

fn bounds(node: &Value) -> [f64; 4] {
    ["x", "y", "width", "height"].map(|key| node[key].as_f64().unwrap())
}

fn computed_bounds(response: &Value, id: &str) -> Vec<[f64; 4]> {
    node(response, id)["computedBounds"]
        .as_array()
        .unwrap()
        .iter()
        .map(bounds)
        .collect()
}

#[tokio::test]
async fn breakpoint_context_inspect_and_http_share_a_scene_at_one_revision() {
    let s = Server::start().await;
    for uri in [
        "skill://opencanvas/props.md",
        "skill://opencanvas/author-frame.md",
    ] {
        let resource = s.rpc("resources/read", json!({"uri":uri})).await;
        let text = resource["contents"][0]["text"].as_str().unwrap();
        assert!(
            text.contains("breakpoints") && text.contains("100%"),
            "{uri}"
        );
    }
    let p = s.project().await;
    let result = s.batch(&p, 0, "responsive", fixture()).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    let authored = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"frame","compact":true,"theme":"dense"}),
        )
        .await;
    assert_eq!(authored["diagnostics"], json!([]), "{authored}");
    let rule = &node(&authored, "frame")["resolvedProps"]["layout"]["breakpoints"][0]["layout"];
    assert_eq!(rule["padding"], 20);
    assert_eq!(authored["tokens"][0]["id"], "padding");
    let fixed = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"fixed"}),
        )
        .await;
    assert_eq!(fixed["diagnostics"][0]["code"], "NO_REFLOW_INFORMATION");

    let viewports = json!([{"width":1200,"height":400},{"width":601,"height":400},
        {"width":600,"height":400},{"width":390,"height":400}]);
    let context = settled(
        &s,
        json!({"projectId":p,"frameId":"frame","theme":"dense","viewports":viewports}),
    )
    .await;
    assert_eq!(context["computed"]["status"], "ready", "{context}");
    assert_eq!(context["reflow"], true);
    assert_eq!(context["revision"], 1);
    assert_eq!(
        computed_bounds(&context, "panel"),
        [
            [40., 30., 600., 80.],
            [40., 30., 300.5, 80.],
            [20., 20., 280., 80.],
            [20., 20., 220., 80.]
        ]
    );
    assert_eq!(
        computed_bounds(&context, "aside"),
        [
            [440., 30., 100., 40.],
            [440., 30., 100., 40.],
            [20., 112., 100., 40.],
            [20., 112., 100., 40.]
        ]
    );
    let details = &node(&context, "detail")["computedBounds"];
    assert_eq!(bounds(&details[2]), [30., 26., 125., 20.]);
    assert_eq!(bounds(&details[3]), [30., 26., 110., 20.]);

    let jobs = context["computed"]["viewports"].as_array().unwrap();
    assert_ne!(jobs[1]["sceneKey"], jobs[2]["sceneKey"]);
    for index in [1, 2] {
        let job = &jobs[index];
        let inspected = s
            .call(
                "inspect_layout",
                json!({"projectId":p,"jobId":job["jobId"],
            "sceneKey":job["sceneKey"],"nodeIds":["detail"],"fields":["bounds"]}),
            )
            .await;
        assert_eq!(inspected["revision"], 1);
        assert_eq!(
            inspected["nodes"][0]["bounds"], details[index],
            "{inspected}"
        );
        let width = viewports[index]["width"].as_u64().unwrap();
        let scene = s.get(&format!("/api/v1/projects/{p}/scene?frameId=frame&revision=1&width={width}&height=400&theme=dense")).await;
        assert_eq!(scene["sceneKey"], job["sceneKey"], "{scene}");
        assert_eq!(node(&scene["scene"], "detail")["bounds"], details[index]);
    }
    let rendered = s
        .call(
            "render_frame",
            json!({"projectId":p,"jobId":jobs[3]["jobId"],"waitMs":10000}),
        )
        .await;
    assert_eq!(rendered["status"], "ready", "{rendered}");
    assert_eq!(rendered["sceneKey"], jobs[3]["sceneKey"]);
    let png = s
        .client
        .get(rendered["imageUrl"].as_str().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(png.headers()["content-type"], "image/png");
    let bytes = png.bytes().await.unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(&bytes[16..20], &390u32.to_be_bytes());
    assert_eq!(&bytes[20..24], &400u32.to_be_bytes());
    let equal = settled(
        &s,
        json!({"projectId":p,"frameId":"frame","theme":"dense","viewports":[
            {"width":390,"height":400},{"width":400,"height":400}
        ]}),
    )
    .await;
    assert_eq!(equal["reflow"], false, "{equal}");
    assert_eq!(equal["diagnostics"][0]["reason"], "UNCHANGED_BOUNDS");
}

#[tokio::test]
async fn invalid_rule_or_hug_axis_rejects_an_entire_mcp_batch() {
    let s = Server::start().await;
    let p = s.project().await;
    assert_eq!(
        s.batch(&p, 0, "initial", fixture()).await["committedRevision"],
        1
    );
    let path = format!("/api/v1/projects/{p}/snapshot");
    let before = s.get(&path).await;
    for update in [
        json!({"op":"update_node","id":"panel","props":{"width":"hug"}}),
        json!({"op":"update_node","id":"frame","props":{"layout":{"mode":"free","breakpoints":[
            {"maxWidth":600,"layout":{"mode":"column","wrap":true}}
        ]}}}),
    ] {
        let response = s
            .batch(
                &p,
                1,
                "atomic",
                json!([
                    {"op":"update_node","id":"aside","props":{"name":"must roll back"}}, update
                ]),
            )
            .await;
        assert_eq!(response["error"]["code"], "VALIDATION_FAILED", "{response}");
        assert_eq!(s.get(&path).await, before);
    }
    let retried = s
        .batch(
            &p,
            1,
            "atomic",
            json!([
                {"op":"update_node","id":"aside","props":{"name":"valid"}}
            ]),
        )
        .await;
    assert_eq!(retried["committedRevision"], 2, "{retried}");
}
