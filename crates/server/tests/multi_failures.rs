mod support;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use support::{design, Server};

fn hanging_worker() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("worker")).unwrap();
    std::fs::write(
        dir.path().join("worker/cli.mjs"),
        "await new Promise(()=>setInterval(()=>{},1000));",
    )
    .unwrap();
    dir
}
async fn commit_direct(s: &Server, p: &str) {
    let req = serde_json::from_value(
        json!({"projectId":p,"expectedRevision":0,"idempotencyKey":"direct","operations":design()}),
    )
    .unwrap();
    let p = p.to_owned();
    s.state
        .db(move |store| store.open_project(&p)?.apply_batch(req))
        .await
        .unwrap();
}
fn args(p: &str) -> Value {
    json!({"projectId":p,"frameId":"frame","viewports":[{"width":320,"height":200},{"width":240,"height":200}]})
}

#[tokio::test]
async fn context_never_waits_for_hanging_worker_and_retains_failure_until_explicit_retry() {
    let workspace = hanging_worker();
    let s = Server::configured(|c| {
        c.workspace = workspace.path().into();
        c.job_timeout = Duration::from_millis(500);
    })
    .await;
    let p = s.project().await;
    commit_direct(&s, &p).await;
    let start = Instant::now();
    let pending = s.call("get_design_context", args(&p)).await;
    assert!(start.elapsed() < Duration::from_millis(450));
    assert_eq!(pending["computed"]["status"], "pending", "{pending}");
    assert!(pending["reflow"].is_null());
    assert_eq!(pending["nodes"][0]["computedBounds"], json!([null, null]));
    for vp in pending["computed"]["viewports"].as_array().unwrap() {
        let failed = s.wait_job(vp["jobId"].as_str().unwrap()).await;
        assert_eq!(failed["error"]["code"], "JOB_TIMEOUT");
    }
    let failed = s.call("get_design_context", args(&p)).await;
    assert_eq!(failed["computed"]["status"], "failed");
    assert!(failed["reflow"].is_null());
    assert_eq!(
        failed["computed"]["viewports"][0]["jobId"],
        pending["computed"]["viewports"][0]["jobId"]
    );
    assert_eq!(
        failed["computed"]["viewports"][0]["error"]["code"],
        "JOB_TIMEOUT"
    );
    let retry = s.call("render_frame", json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":320,"height":200}})).await;
    assert_ne!(retry["jobId"], failed["computed"]["viewports"][0]["jobId"]);
    let retried = s.call("get_design_context", args(&p)).await;
    assert_eq!(retried["computed"]["viewports"][0]["jobId"], retry["jobId"]);
    assert!(retried["reflow"].is_null());
}

#[tokio::test]
async fn full_queue_is_explicit_retryable_and_does_not_fabricate_no_reflow() {
    let workspace = hanging_worker();
    let s = Server::configured(|c| {
        c.workspace = workspace.path().into();
        c.job_timeout = Duration::from_secs(5);
    })
    .await;
    let p = s.project().await;
    commit_direct(&s, &p).await;
    for width in 400..410 {
        s.call("render_frame", json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":width,"height":200}})).await;
    }
    let result = s.call("get_design_context", args(&p)).await;
    assert_eq!(result["computed"]["status"], "failed", "{result}");
    assert!(result["reflow"].is_null());
    assert_eq!(result["truncated"], false);
    for item in result["computed"]["viewports"].as_array().unwrap() {
        assert_eq!(item["status"], "failed");
        assert_eq!(item["error"]["code"], "QUEUE_FULL");
        assert_eq!(item["error"]["retryable"], true);
    }
}
