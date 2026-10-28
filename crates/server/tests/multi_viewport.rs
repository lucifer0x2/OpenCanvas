mod multi_support;
mod support;
use multi_support::{sample, settled, viewports, wait_context};
use serde_json::{json, Value};
use std::time::Instant;
use support::Server;

#[tokio::test]
async fn actual_samples_compare_complete_scope_and_paginate_complete_bounds() {
    let s = Server::start().await;
    for stacks in [false, true] {
        let (p, frame, count) = sample(&s, stacks).await;
        let args = json!({"projectId":p,"frameId":frame,"viewports":viewports(),
            "limit":1,"depth":0,"properties":["name"]});
        let start = Instant::now();
        let first = s.call("get_design_context", args.clone()).await;
        let read_time = start.elapsed();
        assert!(first.get("error").is_none(), "{first}");
        assert!(first["reflow"].is_null(), "{first}");
        let (ready, cold_time) = wait_context(&s, args.clone()).await;
        eprintln!("sample stacks={stacks} enqueue_read={read_time:?} settle={cold_time:?}");
        assert_eq!(ready["computed"]["status"], "ready", "{ready}");
        assert_eq!(ready["reflow"], stacks, "{ready}");
        assert_eq!(ready["computed"]["comparedNodes"], count - 1);
        assert_eq!(ready["nodes"].as_array().unwrap().len(), 1);
        assert_eq!(
            ready["computed"]["viewports"][0]["jobId"],
            first["computed"]["viewports"][0]["jobId"]
        );
        let b = &ready["nodes"][0]["computedBounds"];
        assert_eq!(b[0]["width"], 1040);
        assert_eq!(b[1]["width"], 800);
        if stacks {
            assert_eq!(ready["diagnostics"], json!([]));
        } else {
            assert_eq!(ready["diagnostics"][0]["code"], "NO_REFLOW_INFORMATION");
            assert_eq!(
                ready["diagnostics"][0]["authoredSize"],
                json!({"width":1040,"height":720})
            );
        }
        // Changing the query invalidates cursors, including ordered viewports and theme.
        for (key, value) in [
            (
                "viewports",
                json!([{"width":800,"height":720},{"width":1040,"height":720}]),
            ),
            ("theme", json!("other")),
            ("properties", json!(["width"])),
        ] {
            let mut changed = args.clone();
            changed["cursor"] = ready["nextCursor"].clone();
            changed[key] = value;
            assert_eq!(
                s.call("get_design_context", changed).await["error"]["code"],
                "INVALID_CURSOR"
            );
        }
        let mut paged = json!({"projectId":p,"frameId":frame,"viewports":viewports(),"properties":["name"],"budget":8192});
        let mut seen = std::collections::BTreeSet::new();
        let mut pages = 0;
        let mut any_changed = false;
        loop {
            let page = s.call("get_design_context", paged.clone()).await;
            assert!(page.get("error").is_none(), "{page}");
            assert_eq!(page["reflow"], stacks);
            assert_eq!(page["computed"]["status"], "ready");
            assert!(serde_json::to_vec(&page).unwrap().len() <= 8192);
            for node in page["nodes"].as_array().unwrap() {
                assert!(seen.insert(node["id"].as_str().unwrap().to_owned()));
                assert_eq!(node["computedBounds"].as_array().unwrap().len(), 2);
                if node["id"] != frame {
                    any_changed |= node["computedBounds"][0] != node["computedBounds"][1];
                }
            }
            pages += 1;
            assert!(pages < 40);
            if page["truncated"] == false {
                break;
            }
            assert!(page["nextCursor"].is_string());
            paged["cursor"] = page["nextCursor"].clone();
        }
        assert!(pages > 1);
        assert_eq!(seen.len(), count);
        assert_eq!(any_changed, stacks);
        // The legacy single viewport remains an authored-only read, with the old echo.
        for vp in [None, Some(json!({"width":320,"height":480}))] {
            let mut legacy = json!({"projectId":p,"frameId":frame});
            if let Some(vp) = &vp {
                legacy["viewport"] = vp.clone()
            }
            let old = s.call("get_design_context", legacy).await;
            assert_eq!(old["viewport"], vp.unwrap_or(Value::Null));
            assert!(old.get("computed").is_none());
            assert!(old.get("reflow").is_none());
            assert!(old["nodes"][0].get("computedBounds").is_none());
        }
        let single = settled(
            &s,
            json!({"projectId":p,"frameId":frame,"viewports":[{"width":1040,"height":720}]}),
        )
        .await;
        assert_eq!(single["computed"]["status"], "ready");
        assert!(single["reflow"].is_null());
        let changed = s
            .batch(
                &p,
                1,
                "new-revision",
                json!([{"op":"update_node","id":frame,"props":{"name":"new revision"}}]),
            )
            .await;
        assert_eq!(changed["committedRevision"], 2);
        let mut stale = args.clone();
        stale["cursor"] = ready["nextCursor"].clone();
        assert_eq!(
            s.call("get_design_context", stale).await["error"]["code"],
            "INVALID_CURSOR"
        );
    }
}

#[tokio::test]
async fn unchanged_fixed_stacks_have_honest_diagnostic_and_limits_are_explicit() {
    let s = Server::start().await;
    let p = s.project().await;
    let ops = json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"upsert_token","token":{"id":"color","name":"Color","type":"color","value":{"default":"#ffffff","themes":{"night":"#111111"}}}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":1040,"height":720,"fill":{"tokenId":"color"},"layout":{"mode":"column"}}},
        {"op":"create_node","id":"child","kind":"rectangle","parentId":"frame","props":{"width":50,"height":50}}
    ]);
    assert_eq!(s.batch(&p, 0, "fixed", ops).await["committedRevision"], 1);
    let args = json!({"projectId":p,"frameId":"frame","viewports":viewports()});
    let ready = settled(&s, args.clone()).await;
    assert_eq!(ready["reflow"], false, "{ready}");
    assert_eq!(ready["diagnostics"][0]["reason"], "UNCHANGED_BOUNDS");
    let mut themed = args.clone();
    themed["theme"] = json!("night");
    let night = settled(&s, themed).await;
    assert_eq!(night["reflow"], false);
    assert_ne!(
        night["computed"]["viewports"][0]["sceneKey"],
        ready["computed"]["viewports"][0]["sceneKey"]
    );
    assert!(!ready["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("All scoped nodes use free"));
    for vp in [
        json!([]),
        json!([{"width":-1,"height":20}]),
        json!([{"width":4000,"height":4000},{"width":1,"height":1}]),
        json!(vec![json!({"width":10,"height":10}); 5]),
    ] {
        let mut bad = args.clone();
        bad["viewports"] = vp;
        assert_eq!(
            s.call("get_design_context", bad).await["error"]["code"],
            "INVALID_REQUEST"
        );
    }
    let mut both = args.clone();
    both["viewport"] = json!({"width":320,"height":200});
    assert_eq!(
        s.call("get_design_context", both).await["error"]["code"],
        "INVALID_REQUEST"
    );
    let mut tiny = args.clone();
    tiny["budget"] = json!(1024);
    assert_eq!(
        s.call("get_design_context", tiny).await["error"]["code"],
        "CONTEXT_ITEM_TOO_LARGE"
    );
    let mut four = args;
    four["viewports"] = json!(vec![json!({"width":800,"height":720}); 4]);
    let four = settled(&s, four).await;
    assert_eq!(four["computed"]["status"], "ready");
    assert_eq!(
        four["nodes"][0]["computedBounds"].as_array().unwrap().len(),
        4
    );
    assert_eq!(four["reflow"], false);
}
