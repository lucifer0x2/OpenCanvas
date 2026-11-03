#[allow(dead_code)] // This test uses only the MCP read and batch helpers.
mod support;
use serde_json::{json, Value};
use support::Server;

#[tokio::test]
async fn large_animated_scope_pages_and_has_a_complete_narrow_lookup() {
    let s = Server::start().await;
    let project = s.project().await;
    let source = format!("group-{}", "g".repeat(122));
    let animation = json!({"name":"l".repeat(64),"on":"hover","triggerNodeId":source,
        "duration":200,"delay":30,"keyframes":[{"scale":1.02}]});
    let mut ops = vec![
        json!({"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}}),
        json!({"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":320,"height":200}}),
        json!({"op":"create_node","id":source,"kind":"group","parentId":"frame","props":{}}),
    ];
    let targets: Vec<_> = (0..240)
        .map(|i| format!("target-{i:03}-{}", "t".repeat(117)))
        .collect();
    for target in &targets {
        ops.push(
            json!({"op":"create_node","id":target,"kind":"rectangle","parentId":source,
            "props":{"width":20,"height":20,"animations":[animation]}}),
        );
    }
    let committed = s.batch(&project, 0, "large", json!(ops)).await;
    assert_eq!(committed["committedRevision"], 1, "{committed}");

    let unbounded_summary: Vec<_> = targets
        .iter()
        .map(|id| {
            let name = animation["name"].as_str().unwrap();
            json!({"ref":format!("{id}:{name}"),"on":"hover","triggerNodeId":source})
        })
        .collect();
    assert!(serde_json::to_vec(&unbounded_summary).unwrap().len() > 65536);

    let ctx = json!({"projectId":project,"frameId":"frame","revision":1,"properties":["name"],
        "compact":true,"nodeFormat":"rows","limit":100});
    let pages = all_pages(&s, "get_design_context", ctx.clone(), 65536).await;
    assert!(pages.len() > 1);
    let summary = pages[0]["animated"].as_array().unwrap();
    assert!(!summary.is_empty());
    assert_eq!(summary, &unbounded_summary[..summary.len()]);
    for page in &pages {
        assert_eq!(page["animatedTruncated"], true);
        assert_eq!(page["animated"].as_array().unwrap(), summary);
    }
    assert_eq!(
        pages
            .iter()
            .map(|p| p["nodes"].as_array().unwrap().len())
            .sum::<usize>(),
        targets.len() + 2
    );

    let mut small_ctx = ctx;
    small_ctx["budget"] = json!(1024);
    let small = s.call("get_design_context", small_ctx).await;
    assert_eq!(small["animatedTruncated"], true, "{small}");
    assert!(small.get("animated").is_none(), "{small}");
    assert!(!small["nodes"].as_array().unwrap().is_empty(), "{small}");
    assert!(small["nextCursor"].is_string(), "{small}");
    assert!(serde_json::to_vec(&small).unwrap().len() <= 1024);

    let lookup = json!({"projectId":project,"frameId":"frame","revision":1,
        "properties":["animations"],"kinds":["frame","group","rectangle","text","image"],"limit":40});
    let pages = all_pages(&s, "find_nodes", lookup, 65536).await;
    assert!(pages.len() > 1);
    let mut discovered = Vec::new();
    for node in pages.iter().flat_map(|p| p["matches"].as_array().unwrap()) {
        if let Some(tracks) = node["props"]["animations"].as_array() {
            assert_eq!(tracks, &vec![animation.clone()]);
            discovered.push(node["id"].as_str().unwrap());
        }
    }
    assert_eq!(discovered, targets);
    let source_node = s
        .call(
            "find_nodes",
            json!({"projectId":project,"revision":1,"ids":[source]}),
        )
        .await;
    assert_eq!(source_node["matches"][0]["kind"], "group");
    assert_eq!(source_node["matches"][0]["id"], source);
}

async fn all_pages(s: &Server, tool: &str, mut args: Value, budget: usize) -> Vec<Value> {
    let mut pages = Vec::new();
    loop {
        let page = s.call(tool, args.clone()).await;
        assert!(page.get("error").is_none(), "{page}");
        assert!(serde_json::to_vec(&page).unwrap().len() <= budget);
        args["cursor"] = page["nextCursor"].clone();
        pages.push(page);
        if args["cursor"].is_null() {
            return pages;
        }
    }
}
