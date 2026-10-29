//! P08a over MCP: per-side padding/align/justify read back and observable reflow.
mod multi_support;
mod support;
use multi_support::settled;
use serde_json::{json, Value};
use support::Server;

fn node(id: &str, parent: &str, kind: &str, props: Value) -> Value {
    json!({"op":"create_node","id":id,"kind":kind,"parentId":parent,"props":props})
}

#[tokio::test]
async fn authored_padding_list_and_tokens_read_back_verbatim() {
    let s = Server::start().await;
    let p = s.project().await;
    let layout = json!({"mode":"row","padding":[{"tokenId":"space"},16],
        "align":"center","justify":"space-between"});
    let ops = json!([
        {"op":"upsert_token","token":{"id":"space","name":"Space","type":"dimension","value":12}},
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        node("bar", "page", "frame", json!({"width":400,"height":48,"layout":layout})),
        node("a", "bar", "rectangle", json!({"width":24,"height":24})),
        node("b", "bar", "rectangle", json!({"width":24,"height":24}))
    ]);
    assert_eq!(s.batch(&p, 0, "bar", ops).await["committedRevision"], 1);
    let args = json!({"projectId":p,"frameId":"bar","viewports":[{"width":400,"height":48}]});
    let context = settled(&s, args.clone()).await;
    let nodes = context["nodes"].as_array().unwrap();
    let bar = nodes.iter().find(|n| n["id"] == "bar").unwrap();
    assert_eq!(bar["props"]["layout"], layout);
    assert_eq!(bar["resolvedProps"]["layout"]["padding"], json!([12, 16]));
    let b = nodes.iter().find(|n| n["id"] == "b").unwrap();
    assert_eq!(b["computedBounds"][0]["x"], 360, "{b}");
    assert_eq!(b["computedBounds"][0]["y"], 12);
    let mut compact = args;
    compact["compact"] = json!(true);
    let compact = s.call("get_design_context", compact).await;
    assert_eq!(compact["resources"], "referenced");
    let tokens: Vec<_> = compact["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| &t["id"])
        .collect();
    assert_eq!(tokens, [&json!("space")]);
}

async fn card_bounds(s: &Server, slug: &str, frame_layout: Value, card: Value) -> Value {
    let p = s
        .call("create_project", json!({"slug":slug,"name":"Stack reflow"}))
        .await["projectId"]
        .as_str()
        .unwrap()
        .to_owned();
    let ops = json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        node("frame", "page", "frame", json!({"width":1040,"height":844,"layout":frame_layout})),
        node("card", "frame", "group", card)
    ]);
    assert_eq!(s.batch(&p, 0, "reflow", ops).await["committedRevision"], 1);
    settled(
        s,
        json!({"projectId":p,"frameId":"frame","viewports":[
            {"width":1040,"height":844},{"width":390,"height":844}]}),
    )
    .await
}
fn axis(context: &Value, key: &str) -> Vec<f64> {
    let card = context["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "card")
        .unwrap();
    let bounds = card["computedBounds"].as_array().unwrap();
    bounds.iter().map(|b| b[key].as_f64().unwrap()).collect()
}

#[tokio::test]
async fn centered_stack_reflows_where_the_free_copy_is_a_picture() {
    let s = Server::start().await;
    let stack = card_bounds(
        &s,
        "stack",
        json!({"mode":"column","padding":[24,32],"align":"center","justify":"center"}),
        json!({"width":320,"height":200}),
    )
    .await;
    assert_eq!(stack["reflow"], true, "{stack}");
    assert_eq!(stack["diagnostics"], json!([]));
    assert_eq!(axis(&stack, "x"), [360., 35.]);
    assert_eq!(axis(&stack, "y"), [322., 322.]);
    let free = card_bounds(
        &s,
        "free",
        json!({"mode":"free"}),
        json!({"width":320,"height":200,"x":360,"y":322}),
    )
    .await;
    assert_eq!(
        free["diagnostics"][0]["code"], "NO_REFLOW_INFORMATION",
        "{free}"
    );
}
