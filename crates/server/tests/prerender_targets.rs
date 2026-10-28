//! Task 07F item 4: a commit prerenders the frames that contain its changed nodes (at most 4),
//! not the first four frames in storage order. Observed through `scene_ready` events.
mod support;
use serde_json::{json, Value};
use std::{collections::HashSet, time::Duration};
use support::Server;
use tokio::{sync::broadcast::Receiver, time::timeout};

fn six_frames() -> Value {
    let mut ops = vec![
        json!({"op":"create_node","id":"page","kind":"page","parentId":null,"props":{"name":"Page"}}),
    ];
    for i in 1..=6 {
        let frame = format!("frame-{i}");
        ops.push(json!({"op":"create_node","id":frame,"kind":"frame","parentId":"page","props":{"width":300 + i * 10,"height":200}}));
        ops.push(json!({"op":"create_node","id":format!("{frame}-label"),"kind":"text","parentId":frame,"props":{"text":format!("Frame {i}"),"width":200,"height":"hug","fontId":"font_bundled","fontSize":16,"lineHeight":24}}));
    }
    Value::Array(ops)
}

/// Job IDs of `scene_ready` events for `revision`, until 2 s pass without one (45 s at most).
async fn prerendered(events: &mut Receiver<Value>, revision: u64) -> HashSet<String> {
    let mut jobs = HashSet::new();
    let quiet = |jobs: &HashSet<String>| Duration::from_secs(if jobs.is_empty() { 45 } else { 2 });
    while let Ok(Ok(event)) = timeout(quiet(&jobs), events.recv()).await {
        if event["event"] == "scene_ready" && event["revision"] == revision {
            jobs.insert(event["jobId"].as_str().unwrap().to_owned());
        }
    }
    jobs
}

async fn render(s: &Server, project: &str, revision: u64, i: u64) -> String {
    let viewport = json!({"width":300 + i * 10,"height":200});
    let job = s.call("render_frame", json!({"projectId":project,"revision":revision,"frameId":format!("frame-{i}"),"viewport":viewport})).await;
    job["jobId"]
        .as_str()
        .unwrap_or_else(|| panic!("{job}"))
        .to_owned()
}

#[tokio::test]
async fn editing_the_sixth_frame_prerenders_only_that_frame() {
    let s = Server::start().await;
    let p = s.project().await;
    let mut events = s.state.events.subscribe();
    assert_eq!(
        s.batch(&p, 0, "seed", six_frames()).await["committedRevision"],
        1
    );
    // Every frame changed: the first four, as before (the cap still applies).
    let seeded = prerendered(&mut events, 1).await;
    assert_eq!(seeded.len(), 4, "{seeded:?}");
    for i in 1..=4 {
        assert!(seeded.contains(&render(&s, &p, 1, i).await), "frame-{i}");
    }

    let edit = json!([{"op":"update_node","id":"frame-6-label","props":{"text":"Edited"}}]);
    assert_eq!(s.batch(&p, 1, "edit", edit).await["committedRevision"], 2);
    let jobs = prerendered(&mut events, 2).await;
    assert_eq!(jobs.len(), 1, "{jobs:?}");
    assert!(jobs.contains(&render(&s, &p, 2, 6).await));

    // Token-only change: no node resolves to a frame, so the first four frames as before.
    let token = json!([{"op":"upsert_token","token":{"id":"accent","name":"Accent","type":"color","value":"#ff0000"}}]);
    let committed = s.batch(&p, 2, "token", token).await;
    assert_eq!(committed["committedRevision"], 3, "{committed}");
    let jobs = prerendered(&mut events, 3).await;
    assert_eq!(jobs.len(), 4, "{jobs:?}");
    for i in 1..=4 {
        assert!(jobs.contains(&render(&s, &p, 3, i).await), "frame-{i}");
    }
}
