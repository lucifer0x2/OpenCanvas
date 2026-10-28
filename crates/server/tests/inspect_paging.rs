#[allow(dead_code)] // `design` is unused here.
mod support;
use serde_json::{json, Value};
use std::collections::HashSet;
use support::Server;

const NODES: usize = 250;
const LONG: usize = NODES - 2;

/// A frame with 250 children (alternating rectangle/text), rendered once.
async fn rendered(s: &Server) -> (String, Value) {
    let p = s.project().await;
    let mut ops = vec![
        json!({"op":"create_node","id":"page","kind":"page","parentId":null,"props":{"name":"Home"}}),
        json!({"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":400,"height":3000,"fill":"#ffffff"}}),
    ];
    for i in 0..NODES - 1 {
        // The last child is one long narrow text: dozens of lines, one item > 1 KiB.
        let (text, width) = match i {
            LONG => ("Tiếng Việt ".repeat(60), 60),
            _ => (format!("Dòng {i}"), 300),
        };
        ops.push(if i % 2 == 0 && i != LONG {
            json!({"op":"create_node","id":format!("n{i:03}"),"kind":"rectangle","parentId":"frame","props":{"x":0,"y":i*12,"width":40,"height":10,"fill":"#336699"}})
        } else {
            json!({"op":"create_node","id":format!("n{i:03}"),"kind":"text","parentId":"frame","props":{"x":50,"y":i*12,"text":text,"width":width,"height":"hug","fontId":"font_bundled","fontSize":10,"lineHeight":12}})
        });
    }
    let done = s.batch(&p, 0, "many", Value::Array(ops)).await;
    assert_eq!(done["committedRevision"], 1, "{done}");
    let job = s.call("render_frame",json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":400,"height":3000}})).await;
    let job = s.wait_job(job["jobId"].as_str().unwrap()).await;
    assert_eq!(job["status"], "ready", "{job}");
    assert_eq!(job["scene"]["nodes"].as_array().unwrap().len(), NODES);
    (p, job)
}

/// The pre-Task07B `inspect_layout` body, kept as the byte-identity oracle.
fn legacy(job: &Value, ids: Option<&Vec<Value>>) -> Value {
    let selected: Vec<_> = job["scene"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|n| ids.is_none_or(|ids| ids.contains(&n["id"])))
        .collect();
    let truncated = selected.len() > 200;
    let nodes: Vec<_> = selected
        .into_iter()
        .take(200)
        .map(|n| json!({"id":n["id"],"bounds":n["bounds"],"lines":n["lines"]}))
        .collect();
    json!({"schemaVersion":1,"projectId":job["projectId"],"revision":job["revision"],"sceneKey":job["sceneKey"],"status":job["status"],"nodes":nodes,"diagnostics":job["diagnostics"],"truncated":truncated})
}

fn base(p: &str, job: &Value) -> Value {
    json!({"projectId":p,"jobId":job["jobId"],"sceneKey":job["sceneKey"]})
}

fn with(mut args: Value, extra: Value) -> Value {
    for (k, v) in extra.as_object().unwrap() {
        args[k] = v.clone();
    }
    args
}

/// Follows nextCursor with identical arguments; returns (pages, all nodes).
async fn read_all(s: &Server, args: Value) -> (usize, Vec<Value>) {
    let (mut pages, mut nodes, mut cursor) = (0, Vec::new(), None::<Value>);
    loop {
        let mut call = args.clone();
        if let Some(c) = &cursor {
            call["cursor"] = c.clone();
        }
        let page = s.call("inspect_layout", call).await;
        assert!(page.get("error").is_none(), "{page}");
        pages += 1;
        nodes.extend(page["nodes"].as_array().unwrap().iter().cloned());
        match page.get("nextCursor") {
            Some(c) => {
                assert_eq!(page["truncated"], true, "cursor only when truncated");
                cursor = Some(c.clone());
            }
            None => {
                assert_eq!(page["truncated"], false);
                return (pages, nodes);
            }
        }
    }
}

fn assert_complete(job: &Value, nodes: &[Value]) {
    let want: Vec<_> = job["scene"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].clone())
        .collect();
    let got: Vec<_> = nodes.iter().map(|n| n["id"].clone()).collect();
    let unique: HashSet<_> = got.iter().map(|v| v.to_string()).collect();
    assert_eq!(unique.len(), got.len(), "no duplicates");
    assert_eq!(got, want, "no gaps, scene order");
}

fn code(v: &Value) -> &str {
    v["error"]["code"].as_str().unwrap_or("none")
}

#[tokio::test]
async fn default_bytes_match_legacy_and_cursor_reads_every_node_once() {
    let s = Server::start().await;
    let (p, job) = rendered(&s).await;
    let args = base(&p, &job);

    // Default: byte-identical to v1 apart from nextCursor on a truncated page.
    let first = s.call("inspect_layout", args.clone()).await;
    let mut expected = legacy(&job, None);
    assert_eq!(expected["truncated"], true);
    let cursor = first["nextCursor"].clone();
    assert!(cursor.is_string(), "{first}");
    expected["nextCursor"] = cursor;
    assert_eq!(first.to_string(), expected.to_string());
    let subset = json!(["n001", "n100", "n248", "missing"]);
    let picked = s
        .call(
            "inspect_layout",
            with(args.clone(), json!({ "nodeIds": subset })),
        )
        .await;
    let expected = legacy(&job, subset.as_array());
    assert_eq!(
        picked.to_string(),
        expected.to_string(),
        "no nextCursor key"
    );

    // Default arguments + cursor: 200 then 50.
    let (pages, nodes) = read_all(&s, args.clone()).await;
    assert_eq!(pages, 2);
    assert_complete(&job, &nodes);
    assert_eq!(nodes[1], legacy(&job, None)["nodes"][1]);

    // Small pages by count and by bytes.
    let (pages, nodes) = read_all(&s, with(args.clone(), json!({"limit":100}))).await;
    assert_eq!(pages, 3);
    assert_complete(&job, &nodes);
    let (pages, bounds) = read_all(
        &s,
        with(args.clone(), json!({"fields":["bounds"],"budget":2048})),
    )
    .await;
    assert!(pages > 3, "budget splits pages: {pages}");
    assert_complete(&job, &bounds);
    for (node, full) in bounds.iter().zip(&nodes) {
        assert_eq!(node, &json!({"id":full["id"],"bounds":full["bounds"]}));
    }
}

#[tokio::test]
async fn fields_cursor_binding_and_limits_are_explicit() {
    let s = Server::start().await;
    let (p, job) = rendered(&s).await;
    let args = base(&p, &job);
    let long = format!("n{LONG:03}");

    let lines = s
        .call(
            "inspect_layout",
            with(args.clone(), json!({"fields":["lines"],"nodeIds":[long]})),
        )
        .await;
    let node = &lines["nodes"][0];
    assert_eq!(node.as_object().unwrap().len(), 2, "{node}");
    assert!(node["lines"].as_array().unwrap().len() > 10, "{node}");
    let reordered = s
        .call(
            "inspect_layout",
            with(
                args.clone(),
                json!({"fields":["lines","bounds"],"nodeIds":[long]}),
            ),
        )
        .await;
    let default = s
        .call(
            "inspect_layout",
            with(args.clone(), json!({"nodeIds":[long]})),
        )
        .await;
    assert_eq!(reordered, default, "fields are a set, default is both");

    // One item larger than the budget is an error, never a silent drop.
    let small = with(args.clone(), json!({"nodeIds":[long],"budget":1024}));
    let too_large = s.call("inspect_layout", small).await;
    assert_eq!(code(&too_large), "CONTEXT_ITEM_TOO_LARGE", "{too_large}");
    for bad in [
        json!({"limit":0}),
        json!({"limit":201}),
        json!({"budget":100}),
        json!({"fields":[]}),
        json!({"fields":["props"]}),
    ] {
        let response = s
            .call("inspect_layout", with(args.clone(), bad.clone()))
            .await;
        assert_eq!(code(&response), "INVALID_REQUEST", "{bad}: {response}");
    }

    // The cursor binds job, scene key, nodeIds, fields and page parameters.
    let paged = with(args.clone(), json!({"limit":100,"fields":["bounds"]}));
    let first = s.call("inspect_layout", paged.clone()).await;
    let cursor = first["nextCursor"].clone();
    for changed in [
        json!({"fields":["lines"]}),
        json!({"limit":50}),
        json!({"nodeIds":["n001"]}),
        json!({"budget":65536}),
    ] {
        let call = with(
            with(paged.clone(), changed.clone()),
            json!({ "cursor": cursor }),
        );
        let response = s.call("inspect_layout", call).await;
        assert_eq!(code(&response), "INVALID_CURSOR", "{changed}: {response}");
    }
    let tampered = with(paged.clone(), json!({"cursor":"eyJvZmZzZXQiOjF9"}));
    assert_eq!(
        code(&s.call("inspect_layout", tampered).await),
        "INVALID_CURSOR"
    );

    // A later commit does not invalidate a job cursor: the scene is immutable.
    let next = s
        .batch(
            &p,
            1,
            "later",
            json!([{"op":"update_node","id":"n000","props":{"fill":"#000000"}}]),
        )
        .await;
    assert_eq!(next["committedRevision"], 2, "{next}");
    let second = s
        .call("inspect_layout", with(paged, json!({ "cursor": cursor })))
        .await;
    assert_eq!(second["revision"], 1);
    assert_eq!(second["nodes"][0]["id"], job["scene"]["nodes"][100]["id"]);
    assert_eq!(second["nodes"].as_array().unwrap().len(), 100);
}
