//! `apply_batch` `source`: the published OCD sample runs through the live MCP tool, commits
//! the same content as its golden JSON operations, and edits with pageIds.
#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use support::Server;

const GOLDEN: &str = include_str!("fixtures/ocd-login.json");

async fn guide(s: &Server) -> String {
    let read = s
        .rpc("resources/read", json!({"uri":"skill://opencanvas/ocd.md"}))
        .await;
    let text = read["contents"][0]["text"].as_str().unwrap();
    assert_eq!(text, include_str!("../skills/ocd.md"));
    text.to_owned()
}

fn sample(text: &str, marker: &str) -> String {
    let (_, after) = text.split_once(&format!("<!-- {marker} -->")).unwrap();
    let (_, after) = after.split_once("```ocd\n").unwrap();
    after.split_once("```").unwrap().0.to_owned()
}

fn args(project: &str, revision: u64, key: &str, input: (&str, Value)) -> Value {
    let mut args = json!({"projectId":project,"expectedRevision":revision,"idempotencyKey":key});
    args[input.0] = input.1;
    args
}

#[tokio::test]
async fn guide_sample_dry_runs_commits_and_edits_like_its_golden_operations() {
    let s = Server::start().await;
    let p = s.project().await;
    let text = guide(&s).await;
    let source = json!(sample(&text, "ocd-sample"));
    let golden: Value = serde_json::from_str(GOLDEN).unwrap();

    let mut dry = args(&p, 0, "login", ("source", source.clone()));
    dry["dryRun"] = json!(true);
    let from_source = s.call("apply_batch", dry).await;
    assert_eq!(from_source["dryRun"], true, "{from_source}");
    let mut dry = args(&p, 0, "login", ("operations", golden.clone()));
    dry["dryRun"] = json!(true);
    let from_json = s.call("apply_batch", dry).await;
    // The source result also lists its generated IDs by line.
    let mut from_source = from_source;
    let auto_ids = from_source
        .as_object_mut()
        .unwrap()
        .remove("autoIds")
        .unwrap();
    assert_eq!(
        auto_ids,
        json!({"14":"card-text-8e5db610","18":"btn-text-ddd3d128"})
    );
    assert_eq!(from_source, from_json);

    let committed = s
        .call("apply_batch", args(&p, 0, "login", ("source", source)))
        .await;
    assert_eq!(committed["committedRevision"], 1, "{committed}");
    assert_eq!(committed["changedIds"], from_source["changedIds"]);
    // Same compiled content, so the JSON body replays the source commit.
    let replay = s.batch(&p, 0, "login", golden).await;
    assert_eq!(replay["replayed"], true, "{replay}");

    let mut edit = args(&p, 1, "edit", ("source", json!(sample(&text, "ocd-edit"))));
    edit["pageIds"] = json!(["login"]);
    let edited = s.call("apply_batch", edit).await;
    assert_eq!(edited["committedRevision"], 2, "{edited}");
    assert_eq!(edited["editGuard"]["pageIds"], json!(["login"]));
    let card = s
        .call("get_subtree", json!({"projectId":p,"nodeId":"card"}))
        .await;
    let nodes = card["nodes"].as_array().unwrap();
    let title = nodes.iter().find(|n| n["id"] == "title").unwrap();
    assert_eq!(title["props"]["text"], "Chào mừng trở lại");
    assert_eq!(title["props"]["fontSize"], 28);
    // `@set card gap=24` merged into the stored layout.
    let card = nodes.iter().find(|n| n["id"] == "card").unwrap();
    assert_eq!(
        card["props"]["layout"],
        json!({"mode":"column","padding":32,"gap":24,"align":"stretch"})
    );
    assert!(nodes.iter().all(|n| n["id"] != "pass"));
}

#[tokio::test]
async fn source_errors_and_the_input_rules() {
    let s = Server::start().await;
    let p = s.project().await;
    let parse = s
        .call(
            "apply_batch",
            args(
                &p,
                0,
                "bad",
                ("source", json!("page#p\n  frame#f 10x10 colr=#fff")),
            ),
        )
        .await;
    assert_eq!(
        parse,
        json!({"error":{"code":"SOURCE_PARSE_FAILED","message":"Unknown key `colr`",
            "retryable":false,"field":"source","line":2,"column":17}})
    );
    let suggested = s
        .call(
            "apply_batch",
            args(
                &p,
                0,
                "bad",
                ("source", json!("page#p\n  frame#f color=#fff")),
            ),
        )
        .await;
    assert_eq!(suggested["error"]["suggestion"], "fill", "{suggested}");
    // Storage validation of compiled operations passes through unchanged.
    let storage = s
        .call(
            "apply_batch",
            args(
                &p,
                0,
                "bad",
                ("source", json!("page#p\n  frame#f 10x10 .missing")),
            ),
        )
        .await;
    assert_eq!(storage["error"]["code"], "VALIDATION_FAILED", "{storage}");
    for invalid in [
        args(&p, 0, "bad", ("dryRun", json!(true))),
        args(&p, 0, "bad", ("source", json!(""))),
        args(&p, 0, "bad", ("source", json!(7))),
    ] {
        let result = s.call("apply_batch", invalid.clone()).await;
        assert_eq!(result["error"]["code"], "INVALID_REQUEST", "{invalid}");
    }
    let outline = s.call("get_outline", json!({"projectId":p})).await;
    assert_eq!(outline["revision"], 0);
}

#[tokio::test]
async fn set_merges_a_partial_layout_and_replaces_a_full_one() {
    let s = Server::start().await;
    let p = s.project().await;
    let create = json!("page#p\n  frame#card 200x100 col p=32 gap=16");
    let created = s
        .call("apply_batch", args(&p, 0, "create", ("source", create)))
        .await;
    assert_eq!(created["committedRevision"], 1, "{created}");
    async fn layout(s: &Server, p: &str) -> Value {
        let read = s
            .call("get_subtree", json!({"projectId":p,"nodeId":"card"}))
            .await;
        read["nodes"][0]["props"]["layout"].clone()
    }
    let mut partial = args(
        &p,
        1,
        "gap",
        ("source", json!("@set card gap=8 align=center")),
    );
    partial["pageIds"] = json!(["p"]);
    let merged = s.call("apply_batch", partial).await;
    assert_eq!(merged["committedRevision"], 2, "{merged}");
    let expected = json!({"mode":"column","padding":32,"gap":8,"align":"center"});
    assert_eq!(layout(&s, &p).await, expected);
    let full = r#"@set card layout={"mode":"row","gap":4}"#;
    let replaced = s
        .call(
            "apply_batch",
            args(&p, 2, "layout", ("source", json!(full))),
        )
        .await;
    assert_eq!(replaced["committedRevision"], 3, "{replaced}");
    assert_eq!(layout(&s, &p).await, json!({"mode":"row","gap":4}));
}

#[tokio::test]
async fn published_schema_offers_source_without_requiring_either_input() {
    let s = Server::start().await;
    let tools = s.rpc("tools/list", json!({})).await;
    let tool = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "apply_batch")
        .unwrap();
    let schema = &tool["inputSchema"];
    assert_eq!(schema["properties"]["source"]["type"], "string");
    assert_eq!(schema["properties"]["source"]["maxLength"], 2 * 1024 * 1024);
    assert_eq!(
        schema["required"],
        json!(["projectId", "expectedRevision", "idempotencyKey"])
    );
    assert!(tool["description"].as_str().unwrap().contains("ocd.md"));
}

/// A text under a parent whose breakpoint stretches keeps its width unset, and a numeric
/// `.style` ID validates; both dry-run and commit on temp storage.
#[tokio::test]
async fn breakpoint_parents_and_numeric_styles_validate() {
    let s = Server::start().await;
    let p = s.project().await;
    let source = json!(concat!(
        "@style 123 fill=#111111\n",
        "page#p\n",
        r#"  frame#f 320x200 layout={"mode":"column","align":"center","breakpoints":"#,
        r#"[{"maxWidth":600,"layout":{"mode":"column","align":"stretch"}}]}"#,
        "\n    text#t font=font_bundled size=14/20 \"Hi\"\n    rect#r 10x10 .123\n",
    ));
    let mut dry = args(&p, 0, "bp", ("source", source.clone()));
    dry["dryRun"] = json!(true);
    let dry = s.call("apply_batch", dry).await;
    assert_eq!(dry["dryRun"], true, "{dry}");
    let committed = s
        .call("apply_batch", args(&p, 0, "bp", ("source", source)))
        .await;
    assert_eq!(committed["committedRevision"], 1, "{committed}");
    let tree = s
        .call("get_subtree", json!({"projectId":p,"nodeId":"f"}))
        .await;
    let node = |id: &str| {
        let nodes = tree["nodes"].as_array().unwrap();
        nodes.iter().find(|n| n["id"] == id).unwrap()["props"].clone()
    };
    assert_eq!(
        node("t"),
        json!({"fontId":"font_bundled","fontSize":14,"lineHeight":20,"text":"Hi","height":"hug"})
    );
    assert_eq!(node("r")["styleId"], "123");
}
