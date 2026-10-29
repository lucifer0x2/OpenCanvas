//! B01: commit prerenders must not crowd out requested renders. A run of commits used to fill the
//! single eight-slot queue, so the next render_frame failed with QUEUE_FULL for seconds.
mod support;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use support::{design, Server};
use tokio::time::timeout;

/// A protocol worker that answers every op after `delay_ms`; `None` never answers.
fn worker(delay_ms: Option<u64>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("worker")).unwrap();
    let script = match delay_ms {
        None => "await new Promise(()=>setInterval(()=>{},1000));".to_owned(),
        Some(ms) => format!(
            r#"
import {{ createInterface }} from 'node:readline';
for await (const line of createInterface({{input:process.stdin}})) {{
 const r=JSON.parse(line);
 await new Promise(done=>setTimeout(done,{ms}));
 const result=r.op==='compute'
 ? {{scene:{{schemaVersion:1,nodes:[],provenance:{{revision:r.input.revision}},diagnostics:[]}},readiness:[]}}
 : {{base64:'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=',width:1,height:1,provenance:{{browserVersion:'fake',runtimeVersion:process.version}}}};
 console.log(JSON.stringify({{id:r.id,status:'ready',...result}}));
}}
"#
        ),
    };
    std::fs::write(dir.path().join("worker/cli.mjs"), script).unwrap();
    dir
}
async fn slow_server(dir: &tempfile::TempDir) -> Server {
    Server::configured(|c| {
        c.workspace = dir.path().into();
        c.job_timeout = Duration::from_secs(5);
    })
    .await
}
/// Seeds revision 1, then commits `edits` text edits through MCP apply_batch, each of which
/// prerenders the frame. Returns the final revision.
async fn commit_run(s: &Server, p: &str, edits: u64) -> u64 {
    s.batch(p, 0, "seed", design()).await;
    for revision in 1..=edits {
        let edit =
            json!([{"op":"update_node","id":"text","props":{"text":format!("Edit {revision}")}}]);
        let result = s
            .batch(p, revision, &format!("edit-{revision}"), edit)
            .await;
        assert_eq!(result["committedRevision"], revision + 1, "{result}");
    }
    edits + 1
}
fn render_args(p: &str, revision: u64, width: u64) -> Value {
    json!({"projectId":p,"revision":revision,"frameId":"frame","viewport":{"width":width,"height":200}})
}

#[tokio::test]
async fn render_frame_succeeds_after_twenty_consecutive_commits() {
    let dir = worker(Some(150));
    let s = slow_server(&dir).await;
    let p = s.project().await;
    let revision = commit_run(&s, &p, 24).await;
    let started = Instant::now();
    let job = s.call("render_frame", render_args(&p, revision, 640)).await;
    assert!(job.get("error").is_none(), "{job}");
    let ready = s.wait_job(job["jobId"].as_str().unwrap()).await;
    assert_eq!(ready["status"], "ready", "{ready}");
    // At most the running prerender is ahead of it; FIFO behind 24 prerenders would take ~7 s.
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "{:?}",
        started.elapsed()
    );
    let context = s.call("get_design_context", json!({"projectId":p,"frameId":"frame","viewports":[{"width":320,"height":200},{"width":480,"height":200}]})).await;
    for item in context["computed"]["viewports"].as_array().unwrap() {
        assert_ne!(item["status"], "failed", "{context}");
    }
}

#[tokio::test]
async fn newer_commit_supersedes_a_queued_prerender_of_the_same_frame() {
    let dir = worker(Some(150));
    let s = slow_server(&dir).await;
    let p = s.project().await;
    let mut events = s.state.events.subscribe();
    let revision = commit_run(&s, &p, 20).await;
    let mut ready = Vec::new();
    // Collect prerender completions until 2 s pass without one.
    while let Ok(Ok(event)) = timeout(Duration::from_secs(2), events.recv()).await {
        if event["event"] == "scene_ready" {
            ready.push((event["revision"].as_u64().unwrap(), event["jobId"].clone()));
        }
    }
    let revisions: Vec<u64> = ready.iter().map(|(r, _)| *r).collect();
    // Only prerenders that were already running survive, plus the newest one.
    assert!(ready.len() <= 6, "{revisions:?}");
    let (_, last_job) = ready
        .iter()
        .find(|(r, _)| *r == revision)
        .expect("final revision prerendered");
    // The surviving prerender is the cache hit for the same request at authored size.
    let same = s.call("render_frame", render_args(&p, revision, 320)).await;
    assert_eq!(&same["jobId"], last_job, "{same}");
    assert_eq!(same["status"], "ready");
}

#[tokio::test]
async fn full_requested_tier_returns_retry_after_and_hint() {
    let dir = worker(None);
    let s = slow_server(&dir).await;
    let p = s.project().await;
    s.batch(&p, 0, "seed", design()).await;
    let mut full = None;
    for width in 400..420 {
        let result = s.call("render_frame", render_args(&p, 1, width)).await;
        if result["error"]["code"] == "QUEUE_FULL" {
            full = Some(result["error"].clone());
            break;
        }
        assert_eq!(result["status"], "pending", "{result}");
    }
    let error = full.expect("requested tier fills up");
    assert_eq!(error["retryable"], true);
    let retry_after = error["retryAfterMs"].as_u64().expect("retryAfterMs");
    assert!((100..=10_000).contains(&retry_after), "{error}");
    assert!(
        error["hint"].as_str().unwrap().contains("waitMs"),
        "{error}"
    );
    // Commits keep working and their prerenders never take a requested slot.
    let edit = json!([{"op":"update_node","id":"text","props":{"text":"Still committing"}}]);
    assert_eq!(s.batch(&p, 1, "edit", edit).await["committedRevision"], 2);
    let context = s.call("get_design_context", json!({"projectId":p,"frameId":"frame","viewports":[{"width":500,"height":200},{"width":520,"height":200}]})).await;
    let item = &context["computed"]["viewports"][0];
    assert_eq!(item["error"]["code"], "QUEUE_FULL", "{context}");
    assert!(
        item["error"]["retryAfterMs"].as_u64().is_some(),
        "{context}"
    );
    let response = reqwest::get(format!(
        "{}/api/v1/projects/{p}/scene?frameId=frame&revision=2&width=540&height=200",
        s.url
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), 503);
    let seconds: u64 = response.headers()["retry-after"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=10).contains(&seconds));
}

#[tokio::test]
async fn ready_requested_job_outlives_a_long_run_of_commit_prerenders() {
    let dir = worker(Some(5));
    let s = slow_server(&dir).await;
    let p = s.project().await;
    s.batch(&p, 0, "seed", design()).await;
    let job = s.call("render_frame", render_args(&p, 1, 640)).await;
    let id = job["jobId"].as_str().unwrap().to_owned();
    assert_eq!(s.wait_job(&id).await["status"], "ready");
    // Each commit queues a prerender; superseded ones used to keep a record of the 32.
    for revision in 1..=40 {
        let edit =
            json!([{"op":"update_node","id":"text","props":{"text":format!("Edit {revision}")}}]);
        s.batch(&p, revision, &format!("edit-{revision}"), edit)
            .await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    let read = s
        .call("render_frame", json!({"projectId":p,"jobId":id}))
        .await;
    assert_eq!(read["status"], "ready", "{read}");
    let image = reqwest::get(format!("{}/api/v1/jobs/{id}/image", s.url))
        .await
        .unwrap();
    assert_eq!(image.status(), 200);
}
