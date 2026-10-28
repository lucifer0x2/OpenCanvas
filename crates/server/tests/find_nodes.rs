mod find_support;
#[allow(dead_code)] // Only `sample` is used here.
mod multi_support;
mod support;
use find_support::{all_pages, find, fixture, ids};
use serde_json::{json, Value};
use support::Server;

#[tokio::test]
async fn kinds_list_every_frame_and_paths_locate_matches() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let outline = s
        .call("get_outline", json!({"projectId":p,"depth":1}))
        .await;
    assert_eq!(outline["truncated"], true, "depth cuts the outline page");
    let frames = find(&s, json!({"projectId":p,"kinds":["frame"]})).await;
    assert_eq!(ids(&frames["matches"]), ["hero", "pricing", "footer"]);
    assert_eq!(
        (
            frames["totalMatches"].as_u64(),
            frames["scannedNodes"].as_u64()
        ),
        (Some(3), Some(22))
    );
    assert_eq!(
        frames["matches"][1]["path"],
        json!([{"id":"page-2","kind":"page","name":"Second"}])
    );
    assert_eq!(frames["matches"][1]["matchedOn"], json!(["kind"]));
    let price = find(
        &s,
        json!({"projectId":p,"nodeId":"price-card","nameContains":"PRICE","kinds":["text"]}),
    )
    .await;
    assert_eq!(price["matches"][0]["id"], "price");
    assert_eq!(
        price["matches"][0]["path"],
        json!([{"id":"price-card","kind":"group","name":"Price card"}])
    );
    assert_eq!(price["matches"][0]["matchedOn"], json!(["name", "kind"]));
    assert_eq!(
        price["criteria"],
        json!({"nameContains":"PRICE","kinds":["text"]})
    );
    let deep = find(&s, json!({"projectId":p,"ids":["deep-text","missing"]})).await;
    let hit = &deep["matches"][0];
    assert_eq!(
        (deep["totalMatches"].as_u64(), hit["depth"].as_u64()),
        (Some(1), Some(12))
    );
    assert_eq!(hit["pathTruncated"], true);
    let path = ids(&hit["path"]);
    assert_eq!(path, ["d3", "d4", "d5", "d6", "d7", "d8", "d9", "d10"]);
    assert_eq!(deep["criteria"], json!({"idCount":2}));
    let scoped = find(
        &s,
        json!({"projectId":p,"frameId":"pricing","kinds":["text"]}),
    )
    .await;
    assert_eq!(ids(&scoped["matches"]), ["pricing-text", "long"]);
    assert_eq!(scoped["scannedNodes"], 3);
    assert_eq!(scoped["matches"][0]["path"][0]["id"], "pricing");
    assert!(scoped["matches"][0].get("pathTruncated").is_none());
}

#[tokio::test]
async fn zero_one_page_and_many_pages_are_distinct() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let none = find(&s, json!({"projectId":p,"nameContains":"absent"})).await;
    assert_eq!(
        (none["totalMatches"].as_u64(), none["truncated"].as_bool()),
        (Some(0), Some(false))
    );
    assert_eq!(
        (none["matches"].clone(), none["nextCursor"].clone()),
        (json!([]), Value::Null)
    );
    let whole = find(&s, json!({"projectId":p,"kinds":["text"],"limit":5})).await;
    assert_eq!(
        (whole["totalMatches"].as_u64(), whole["truncated"].as_bool()),
        (Some(5), Some(false))
    );
    assert!(whole["nextCursor"].is_null());
    let outline: Vec<Value> = all_pages(&s, "get_outline", json!({"projectId":p})).await;
    let preorder: Vec<&str> = outline
        .iter()
        .flat_map(|page| page["nodes"].as_array().unwrap())
        .filter(|n| n["kind"] == "text")
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids(&whole["matches"]), preorder);
    let pages = all_pages(
        &s,
        "find_nodes",
        json!({"projectId":p,"kinds":["text"],"limit":1}),
    )
    .await;
    assert_eq!(pages.len(), 5);
    let mut paged = Vec::new();
    for page in &pages {
        assert_eq!(page["totalMatches"], 5);
        assert_eq!(page["truncated"], !page["nextCursor"].is_null());
        paged.extend(ids(&page["matches"]));
    }
    assert_eq!(paged, preorder);
}

#[tokio::test]
async fn properties_preview_and_case_insensitive_text() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let found = find(
        &s,
        json!({"projectId":p,"textContains":"tổng QUAN","properties":["text","fill"]}),
    )
    .await;
    let hit = &found["matches"][0];
    assert_eq!(
        (found["totalMatches"].as_u64(), hit["id"].as_str()),
        (Some(1), Some("title"))
    );
    assert_eq!(
        hit["props"],
        json!({"text":"Tổng quan doanh thu","fill":"#111111"})
    );
    assert_eq!(hit["textPreview"], "Tổng quan doanh thu");
    let bare = find(&s, json!({"projectId":p,"kinds":["frame"]})).await;
    assert!(bare["matches"][0].get("props").is_none());
    assert!(bare["matches"][0].get("textPreview").is_none());
    let unaccented = find(&s, json!({"projectId":p,"textContains":"Tong quan"})).await;
    assert_eq!(unaccented["totalMatches"], 0, "no accent folding");
    let long = find(&s, json!({"projectId":p,"textContains":"NEEDLE"})).await;
    let preview = long["matches"][0]["textPreview"].as_str().unwrap();
    assert_eq!(preview.chars().count(), 120);
    assert!(preview.ends_with("needle"));
    let names = find(
        &s,
        json!({"projectId":p,"nameContains":"price","properties":["name"]}),
    )
    .await;
    assert_eq!(ids(&names["matches"]), ["price-card", "price"]);
    assert_eq!(names["matches"][1]["props"], json!({"name":"price"}));
}

#[tokio::test]
async fn invalid_requests_use_existing_error_codes() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let code = |v: Value| v["error"]["code"].as_str().unwrap_or("none").to_owned();
    for args in [
        json!({"projectId":p}),
        json!({"projectId":p,"nodeId":"hero","frameId":"hero","kinds":["text"]}),
        json!({"projectId":p,"frameId":"missing","kinds":["text"]}),
        json!({"projectId":p,"frameId":"title","kinds":["text"]}),
        json!({"projectId":p,"kinds":["button"]}),
        json!({"projectId":p,"kinds":[]}),
        json!({"projectId":p,"nameContains":""}),
        json!({"projectId":p,"kinds":["text"],"depth":1}),
        json!({"projectId":p,"ids":["a"],"properties":vec!["k"; 21]}),
    ] {
        assert_eq!(
            code(s.call("find_nodes", args.clone()).await),
            "INVALID_REQUEST",
            "{args}"
        );
    }
    let first = find(&s, json!({"projectId":p,"kinds":["text"],"limit":1})).await;
    let other = json!({"projectId":p,"kinds":["frame"],"limit":1,"cursor":first["nextCursor"]});
    assert_eq!(code(s.call("find_nodes", other).await), "INVALID_CURSOR");
    let stale = json!({"projectId":p,"kinds":["text"],"revision":0});
    assert_eq!(
        code(s.call("find_nodes", stale).await),
        "REVISION_UNAVAILABLE"
    );
    let tight = json!({"projectId":p,"ids":["long"],"properties":["text"],"budget":1024});
    assert_eq!(
        code(s.call("find_nodes", tight).await),
        "CONTEXT_ITEM_TOO_LARGE"
    );
    s.batch(
        &p,
        1,
        "rename",
        json!([{"op":"update_node","id":"title","props":{"name":"x"}}]),
    )
    .await;
    let resumed = json!({"projectId":p,"kinds":["text"],"limit":1,"cursor":first["nextCursor"]});
    assert_eq!(code(s.call("find_nodes", resumed).await), "INVALID_CURSOR");
}

#[tokio::test]
async fn stacks_sample_text_in_one_call() {
    let s = Server::start().await;
    let (p, frame, _) = multi_support::sample(&s, true).await;
    let found = find(
        &s,
        json!({"projectId":p,"frameId":frame,"textContains":"Tổng quan"}),
    )
    .await;
    let hit = &found["matches"][0];
    assert_eq!(
        (hit["kind"].as_str(), hit["path"][0]["id"].as_str()),
        (Some("text"), Some(frame.as_str()))
    );
    assert!(hit["textPreview"].as_str().unwrap().contains("Tổng quan"));
    let outline = all_pages(&s, "get_outline", json!({"projectId":p,"frameId":frame})).await;
    let bytes = |v: &Value| serde_json::to_vec(v).unwrap().len();
    let outline_bytes: usize = outline.iter().map(bytes).sum();
    eprintln!(
        "stacks find_nodes: totalMatches={} scannedNodes={} bytes={}; full frame outline: pages={} bytes={}",
        found["totalMatches"], found["scannedNodes"], bytes(&found), outline.len(), outline_bytes
    );
    assert!(bytes(&found) < outline_bytes);
}
