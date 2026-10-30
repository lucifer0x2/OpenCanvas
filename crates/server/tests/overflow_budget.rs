//! Measured overflow advisories never make a budgeted read fail on their own.
#[allow(dead_code)]
mod multi_support;
#[allow(dead_code)]
mod support;
use multi_support::{overflow, overflow_bytes, settled};
use serde_json::{json, Value};
use support::Server;

const LABELS: usize = 14;

async fn project(s: &Server) -> String {
    let p = s.project().await;
    let mut operations = vec![
        json!({"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}}),
        json!({"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":400,"height":300,"clip":true}}),
    ];
    for i in 0..LABELS {
        operations.push(json!({"op":"create_node","id":format!("label-{i}"),"kind":"text","parentId":"frame",
            "props":{"text":"Overflowing label copy","textWrap":"nowrap","x":10,"y":i * 20,"width":20,"height":8,
                "fontId":"font_bundled","fontSize":12,"lineHeight":16}}));
    }
    let result = s.batch(&p, 0, "labels", json!(operations)).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    p
}

/// Pages a whole context read, asserting every page fits and repeats the same diagnostics.
async fn read_all(s: &Server, mut args: Value, budget: usize) -> Value {
    let mut first: Option<Value> = None;
    let mut seen = 0;
    loop {
        let page = s.call("get_design_context", args.clone()).await;
        assert!(page.get("error").is_none(), "budget {budget}: {page}");
        assert_eq!(page["computed"]["status"], "ready", "{page}");
        assert!(serde_json::to_vec(&page).unwrap().len() <= budget);
        assert!(
            overflow_bytes(&page) <= budget / 4,
            "{}",
            page["diagnostics"]
        );
        let fixed = first.get_or_insert_with(|| page.clone());
        assert_eq!(page["diagnostics"], fixed["diagnostics"]);
        seen += page["nodes"].as_array().unwrap().len();
        if page["truncated"] == false {
            break;
        }
        args["cursor"] = page["nextCursor"].clone();
    }
    assert_eq!(seen, LABELS + 1);
    first.unwrap()
}

#[tokio::test]
async fn many_viewports_of_overflow_compact_instead_of_failing_the_read() {
    let s = Server::start().await;
    let p = project(&s).await;
    let viewports = json!([{"width":400,"height":300},{"width":390,"height":300},{"width":380,"height":300},{"width":370,"height":300}]);
    let args = json!({"projectId":p,"frameId":"frame","viewports":viewports,"properties":["name"]});
    let full = settled(&s, args.clone()).await;
    // Uncompacted, the per-job advisories alone exceed the budgets read below.
    let mut uncompacted = 0;
    for vp in full["computed"]["viewports"].as_array().unwrap() {
        let job = s.wait_job(vp["jobId"].as_str().unwrap()).await;
        let measured: Vec<_> = job["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| {
                d["code"]
                    .as_str()
                    .is_some_and(|c| c.starts_with("TEXT_") && c != "TEXT_FRAGMENTATION_SUSPECTED")
            })
            .collect();
        uncompacted += serde_json::to_vec(&measured).unwrap().len();
    }
    eprintln!("4 viewports: uncompacted overflow bytes={uncompacted}");
    assert!(uncompacted > 8192, "{uncompacted}");

    let mut budgeted = args.clone();
    budgeted["budget"] = json!(8192);
    let page = read_all(&s, budgeted, 8192).await;
    let (items, summaries) = overflow(&page);
    assert_eq!(summaries.len(), 4, "{}", page["diagnostics"]);
    for (summary, vp) in summaries
        .iter()
        .zip(page["computed"]["viewports"].as_array().unwrap())
    {
        assert_eq!(summary["sourceSceneKey"], vp["sceneKey"]);
        assert!(summary.get("provenance").is_none());
        let reported = summary["reportedNodes"].as_u64().unwrap();
        assert_eq!(
            reported + summary["omittedNodes"].as_u64().unwrap(),
            LABELS as u64
        );
        assert!(summary["compaction"]["droppedNodes"].as_u64().unwrap() > 0);
        assert!(summary["compaction"]["retrieve"]
            .as_str()
            .unwrap()
            .contains("inspect_layout"));
        let own = items
            .iter()
            .filter(|i| i["sourceSceneKey"] == vp["sceneKey"])
            .count();
        assert_eq!(own as u64, reported);
    }

    // A small budget collapses to one read-wide summary and still pages every node.
    let mut tiny = args;
    tiny["budget"] = json!(4096);
    let page = read_all(&s, tiny, 4096).await;
    let (items, summaries) = overflow(&page);
    assert!(items.is_empty());
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0]["compaction"]["step"], "summary");
    assert_eq!(summaries[0]["omittedNodes"], 4 * LABELS);

    // inspect_layout with an explicit budget compacts the same way; without one it is whole.
    let vp = &full["computed"]["viewports"][0];
    let inspect = json!({"projectId":p,"jobId":vp["jobId"],"sceneKey":vp["sceneKey"],"fields":["bounds"],"budget":1024,"limit":1});
    let page = s.call("inspect_layout", inspect).await;
    assert!(page.get("error").is_none(), "{page}");
    assert!(serde_json::to_vec(&page).unwrap().len() <= 1024);
    let (_, summaries) = overflow(&page);
    assert_eq!(summaries.len(), 1);
    assert!(summaries[0]["compaction"].is_object());
    let whole =
        json!({"projectId":p,"jobId":vp["jobId"],"sceneKey":vp["sceneKey"],"fields":["bounds"]});
    let whole = s.call("inspect_layout", whole).await;
    let (items, summaries) = overflow(&whole);
    assert_eq!((items.len(), summaries.len()), (12, 1));
    assert_eq!(summaries[0]["provenance"]["sourceSceneKey"], vp["sceneKey"]);
}
