//! `render_frame {jobId, waitMs}`: bounded wait for a job to leave `pending`. A deterministic
//! protocol process stands in for Chromium so each outcome (ready, failed, still pending) is
//! controlled; real rendering is exercised in render.rs.
#[allow(dead_code)] // `start`, `get` and `wait_job` are unused here.
mod support;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use support::{design, Server};

/// `slow`: answer every op after 400 ms. `hang`: never answer.
fn worker(mode: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("worker")).unwrap();
    let script = format!(
        r#"
import {{ createInterface }} from 'node:readline';
const mode={mode:?};
if (mode==='hang') await new Promise(()=>{{setInterval(()=>{{}},1000);}});
for await (const line of createInterface({{input:process.stdin}})) {{
 const r=JSON.parse(line);
 await new Promise((resolve)=>setTimeout(resolve,400));
 const result=r.op==='compute'
 ? {{scene:{{schemaVersion:1,nodes:[],provenance:{{revision:r.input.revision}},diagnostics:[]}},readiness:[]}}
 : {{base64:'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=',width:1,height:1,provenance:{{browserVersion:'fake',runtimeVersion:process.version}}}};
 console.log(JSON.stringify({{id:r.id,status:'ready',...result}}));
}}
"#
    );
    std::fs::write(dir.path().join("worker/cli.mjs"), script).unwrap();
    let workspace = dir.path().to_path_buf();
    (dir, workspace)
}
/// A server on a fake worker with one committed frame (revision 1), committed through storage
/// so no automatic prerender occupies the worker.
async fn server(mode: &str, job_timeout: Duration) -> (tempfile::TempDir, Server, String) {
    let (dir, workspace) = worker(mode);
    let s = Server::configured(|c| {
        c.workspace = workspace;
        c.job_timeout = job_timeout;
    })
    .await;
    let p = s.project().await;
    let req = serde_json::from_value(
        json!({"projectId":p,"expectedRevision":0,"idempotencyKey":"first","operations":design()}),
    )
    .unwrap();
    let id = p.clone();
    s.state
        .db(move |store| store.open_project(&id)?.apply_batch(req))
        .await
        .unwrap();
    (dir, s, p)
}
async fn enqueue(s: &Server, p: &str) -> String {
    let job = s.call("render_frame",json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":320,"height":200}})).await;
    assert_eq!(job["status"], "pending", "{job}");
    job["jobId"].as_str().unwrap().to_owned()
}
async fn poll(s: &Server, p: &str, job: &str, wait: Value) -> (Value, Duration) {
    let started = Instant::now();
    let mut args = json!({"projectId":p,"jobId":job});
    if !wait.is_null() {
        args["waitMs"] = wait;
    }
    let result = s.call("render_frame", args).await;
    (result, started.elapsed())
}

#[tokio::test]
async fn wait_returns_as_soon_as_the_job_is_ready() {
    let (_dir, s, p) = server("slow", Duration::from_secs(30)).await;
    let job = enqueue(&s, &p).await;
    let (ready, took) = poll(&s, &p, &job, json!(10_000)).await;
    assert_eq!(ready["status"], "ready", "{ready}");
    assert!(ready.get("imageUrl").is_some(), "{ready}");
    // compute + render take ~800 ms plus process start; far from the 10 s bound.
    assert!(took >= Duration::from_millis(500), "{took:?}");
    assert!(took < Duration::from_secs(5), "{took:?}");
    // The completed record is the same one an immediate poll returns.
    let (again, _) = poll(&s, &p, &job, Value::Null).await;
    assert_eq!(again, ready);
    let (waited_again, took) = poll(&s, &p, &job, json!(10_000)).await;
    assert_eq!(waited_again, ready);
    assert!(took < Duration::from_secs(1), "{took:?}");
}

#[tokio::test]
async fn wait_returns_as_soon_as_the_job_fails() {
    let (_dir, s, p) = server("hang", Duration::from_millis(600)).await;
    let job = enqueue(&s, &p).await;
    let (failed, took) = poll(&s, &p, &job, json!(10_000)).await;
    assert_eq!(failed["status"], "failed", "{failed}");
    assert_eq!(failed["error"]["code"], "JOB_TIMEOUT");
    assert!(took >= Duration::from_millis(300), "{took:?}");
    assert!(took < Duration::from_secs(5), "{took:?}");
}

#[tokio::test]
async fn wait_expires_with_the_current_pending_state() {
    let (_dir, s, p) = server("hang", Duration::from_secs(30)).await;
    let job = enqueue(&s, &p).await;
    let (immediate, took) = poll(&s, &p, &job, Value::Null).await;
    assert_eq!(immediate["status"], "pending");
    assert!(took < Duration::from_millis(250), "{took:?}");
    // waitMs 0 is the immediate poll, byte for byte.
    let (zero, _) = poll(&s, &p, &job, json!(0)).await;
    assert_eq!(
        serde_json::to_string(&zero).unwrap(),
        serde_json::to_string(&immediate).unwrap()
    );
    let (pending, took) = poll(&s, &p, &job, json!(300)).await;
    assert_eq!(pending, immediate);
    assert!(took >= Duration::from_millis(300), "{took:?}");
    assert!(took < Duration::from_secs(2), "{took:?}");
}

#[tokio::test]
async fn waiting_holds_no_storage_and_ends_on_shutdown() {
    let (_dir, s, p) = server("hang", Duration::from_secs(30)).await;
    let job = enqueue(&s, &p).await;
    let s = std::sync::Arc::new(s);
    let waiter = {
        let (s, p, job) = (s.clone(), p.clone(), job.clone());
        tokio::spawn(async move { poll(&s, &p, &job, json!(10_000)).await })
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    // Reads and commits proceed while a wait is parked.
    let started = Instant::now();
    let outline = s.call("get_outline", json!({"projectId":p})).await;
    assert_eq!(outline["revision"], 1, "{outline}");
    let committed = s
        .batch(
            &p,
            1,
            "while-waiting",
            json!([{"op":"update_node","id":"frame","props":{"fill":"#000000"}}]),
        )
        .await;
    assert_eq!(committed["committedRevision"], 2, "{committed}");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!waiter.is_finished());
    s.state.begin_shutdown();
    let (pending, took) = tokio::time::timeout(Duration::from_secs(3), waiter)
        .await
        .expect("wait must end promptly on shutdown")
        .unwrap();
    assert_eq!(pending["status"], "pending", "{pending}");
    assert!(took < Duration::from_secs(3), "{took:?}");
    // Once shutting down, a new wait does not park at all.
    let (pending, took) = poll(&s, &p, &job, json!(10_000)).await;
    assert_eq!(pending["status"], "pending");
    assert!(took < Duration::from_secs(1), "{took:?}");
}

#[tokio::test]
async fn wait_ms_is_bounded_and_only_for_job_polls() {
    let (_dir, s, p) = server("hang", Duration::from_secs(30)).await;
    let job = enqueue(&s, &p).await;
    for wait in [
        json!(10_001),
        json!(-1),
        json!(1.5),
        json!("100"),
        json!(null),
    ] {
        let result = s
            .call(
                "render_frame",
                json!({"projectId":p,"jobId":job,"waitMs":wait}),
            )
            .await;
        assert_eq!(
            result["error"]["code"], "INVALID_REQUEST",
            "{wait}: {result}"
        );
    }
    let enqueue_with_wait = s.call("render_frame",json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":321,"height":200},"waitMs":100})).await;
    assert_eq!(enqueue_with_wait["error"]["code"], "INVALID_REQUEST");
    // Another project's job is rejected before any waiting.
    let other = s
        .call("create_project", json!({"slug":"other","name":"Other"}))
        .await["projectId"]
        .as_str()
        .unwrap()
        .to_owned();
    let (rejected, took) = poll(&s, &other, &job, json!(10_000)).await;
    assert_eq!(rejected["error"]["code"], "INVALID_REQUEST", "{rejected}");
    assert!(took < Duration::from_secs(1), "{took:?}");
    let (missing, took) = poll(&s, &p, "job_missing", json!(10_000)).await;
    assert_eq!(missing["error"]["code"], "JOB_NOT_FOUND", "{missing}");
    assert!(took < Duration::from_secs(1), "{took:?}");
}
