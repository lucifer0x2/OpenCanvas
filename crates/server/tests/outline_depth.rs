//! B03: `depth` is a filter counted from the scope root, so one call lists a page's root frames,
//! and `find_nodes` `parentId` lists the direct children of one node.
#[allow(dead_code)] // Only `start`, `call`, `rpc`, `project` and `batch` are used here.
mod support;
use serde_json::{json, Value};
use support::Server;

const FRAMES: [&str; 6] = [
    "welcome",
    "connect",
    "grid",
    "structure",
    "console",
    "settings",
];

fn node(id: &str, kind: &str, parent: Option<&str>) -> Value {
    json!({"op":"create_node","id":id,"kind":kind,"parentId":parent,"props":{"name":id,"width":300,"height":200}})
}

/// Page `screens` holds six root frames, each nested four levels deep (group → group → group →
/// rectangle); `welcome` also holds a frame child. Page `other` has one root frame.
async fn fixture(s: &Server) -> String {
    let p = s.project().await;
    let mut ops = vec![node("screens", "page", None)];
    for frame in FRAMES {
        ops.push(node(frame, "frame", Some("screens")));
        let mut parent = frame.to_owned();
        for level in 1..=3 {
            let id = format!("{frame}_{level}");
            ops.push(node(&id, "group", Some(&parent)));
            parent = id;
        }
        ops.push(node(&format!("{frame}_leaf"), "rectangle", Some(&parent)));
    }
    ops.push(node("welcome_card", "frame", Some("welcome")));
    ops.push(node("other", "page", None));
    ops.push(node("other_frame", "frame", Some("other")));
    let result = s.batch(&p, 0, "fixture", json!(ops)).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    p
}

async fn read(s: &Server, tool: &str, args: Value) -> Value {
    let result = s.call(tool, args).await;
    assert!(result.get("error").is_none(), "{result}");
    result
}

fn ids(list: &Value) -> Vec<&str> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn outline_depth_one_lists_every_root_frame_in_one_call() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let page = read(
        &s,
        "get_outline",
        json!({"projectId":p,"nodeId":"screens","depth":1}),
    )
    .await;
    assert_eq!(
        (page["truncated"].as_bool(), page["nextCursor"].clone()),
        (Some(false), Value::Null)
    );
    let mut expected = vec!["screens"];
    expected.extend(FRAMES);
    assert_eq!(ids(&page["nodes"]), expected);
    assert_eq!(page["limits"]["depth"], 1);
    // Welcome has its group and the nested frame; every other frame only its group.
    let omitted: Vec<_> = page["nodes"].as_array().unwrap()[1..]
        .iter()
        .map(|n| n["omittedChildren"].as_u64())
        .collect();
    assert_eq!(
        omitted,
        [Some(2), Some(1), Some(1), Some(1), Some(1), Some(1)]
    );
    assert!(page["nodes"][0].get("omittedChildren").is_none());

    let project = read(&s, "get_outline", json!({"projectId":p,"depth":1})).await;
    assert_eq!(project["truncated"], false);
    let mut expected = vec!["screens"];
    expected.extend(FRAMES);
    expected.extend(["other", "other_frame"]);
    assert_eq!(ids(&project["nodes"]), expected);

    let pages = read(&s, "get_outline", json!({"projectId":p,"depth":0})).await;
    assert_eq!(ids(&pages["nodes"]), ["screens", "other"]);
    assert_eq!(pages["nodes"][0]["omittedChildren"], 6);
}

#[tokio::test]
async fn depth_counts_from_the_scope_root_across_pages_and_tools() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    // Two levels below a root frame, one node per page: the cursor never leaves the depth.
    let mut args = json!({"projectId":p,"nodeId":"grid","depth":2,"limit":1});
    let mut seen = Vec::new();
    loop {
        let page = read(&s, "get_outline", args.clone()).await;
        seen.extend(page["nodes"].as_array().unwrap().iter().cloned());
        if page["truncated"] == false {
            break;
        }
        args["cursor"] = page["nextCursor"].clone();
    }
    assert_eq!(ids(&json!(seen)), ["grid", "grid_1", "grid_2"]);
    assert_eq!(seen[2]["omittedChildren"], 1);
    let subtree = read(
        &s,
        "get_subtree",
        json!({"projectId":p,"nodeId":"grid","depth":1}),
    )
    .await;
    assert_eq!(ids(&subtree["nodes"]), ["grid", "grid_1"]);
    assert_eq!(subtree["nodes"][1]["omittedChildren"], 1);
    let context = read(
        &s,
        "get_design_context",
        json!({"projectId":p,"frameId":"grid","depth":0}),
    )
    .await;
    assert_eq!(ids(&context["nodes"]), ["grid"]);
    // Omitted depth still reads the whole scope, as before.
    let whole = read(&s, "get_outline", json!({"projectId":p,"nodeId":"grid"})).await;
    assert_eq!(
        ids(&whole["nodes"]),
        ["grid", "grid_1", "grid_2", "grid_3", "grid_leaf"]
    );
    assert_eq!(whole["limits"]["depth"], Value::Null);
}

#[tokio::test]
async fn find_nodes_parent_id_lists_direct_children_only() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let frames = read(
        &s,
        "find_nodes",
        json!({"projectId":p,"parentId":"screens","kinds":["frame"]}),
    )
    .await;
    assert_eq!(ids(&frames["matches"]), FRAMES);
    assert_eq!(
        (
            frames["totalMatches"].as_u64(),
            frames["truncated"].as_bool()
        ),
        (Some(6), Some(false))
    );
    assert_eq!(frames["matches"][0]["matchedOn"], json!(["kind", "parent"]));
    assert_eq!(
        frames["criteria"],
        json!({"kinds":["frame"],"parentId":"screens"})
    );
    // Alone, parentId is a valid criterion and returns every kind of direct child.
    let children = read(
        &s,
        "find_nodes",
        json!({"projectId":p,"parentId":"welcome"}),
    )
    .await;
    assert_eq!(ids(&children["matches"]), ["welcome_1", "welcome_card"]);
    let scoped = read(
        &s,
        "find_nodes",
        json!({"projectId":p,"nodeId":"other","parentId":"screens"}),
    )
    .await;
    assert_eq!(scoped["totalMatches"], 0);
    let missing = read(&s, "find_nodes", json!({"projectId":p,"parentId":"nope"})).await;
    assert_eq!(missing["totalMatches"], 0);
    let tools = s.rpc("tools/list", json!({})).await;
    let find = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "find_nodes")
        .unwrap();
    assert_eq!(
        find["inputSchema"]["properties"]["parentId"]["type"],
        "string"
    );
    assert_eq!(find["inputSchema"]["required"], json!(["projectId"]));
}
