mod support;
use serde_json::{json, Value};
use std::time::Duration;
use support::{design, Server};

// A deterministic protocol process isolates server orchestration from Chromium.
// Real rendering/resources are exercised separately in render.rs.
fn cli(mode: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("worker")).unwrap();
    let script = format!(
        r#"
import {{ existsSync,writeFileSync }} from 'node:fs';
import {{ createInterface }} from 'node:readline';
const marker=new URL('../started',import.meta.url);
const first=!existsSync(marker);
writeFileSync(marker,'started');
const mode={mode:?};
if (first && mode==='crash') process.exit(3);
if ((first && mode==='timeout') || mode==='hang') await new Promise(()=>{{setInterval(()=>{{}},1000);}});
for await (const line of createInterface({{input:process.stdin}})) {{
 const r=JSON.parse(line);
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
async fn render(s: &Server, p: &str, width: u64) -> Value {
    s.call("render_frame",json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":width,"height":200}})).await
}
async fn recovery(mode: &str, code: &str) {
    let (_dir, workspace) = cli(mode);
    let s = Server::configured(|c| {
        c.workspace = workspace;
        c.job_timeout = Duration::from_millis(500);
    })
    .await;
    let p = s.project().await;
    // Commit through storage to avoid automatic compute; make the first job deterministic.
    let req = serde_json::from_value(
        json!({"projectId":p,"expectedRevision":0,"idempotencyKey":"first","operations":design()}),
    )
    .unwrap();
    let id = p.clone();
    s.state
        .db(move |store| store.open_project(&id)?.apply_batch(req))
        .await
        .unwrap();
    let first = render(&s, &p, 320).await;
    let failed = s.wait_job(first["jobId"].as_str().unwrap()).await;
    assert_eq!(failed["status"], "failed", "{failed}");
    assert_eq!(failed["error"]["code"], code);
    assert_eq!(failed["error"]["retryable"], true);
    let second = render(&s, &p, 320).await;
    assert_ne!(second["jobId"], first["jobId"]);
    let ready = s.wait_job(second["jobId"].as_str().unwrap()).await;
    assert_eq!(ready["status"], "ready", "{ready}");
    assert!(ready.get("imageUrl").is_some());
    assert_eq!(
        s.get(&format!(
            "/api/v1/jobs/{}",
            first["jobId"].as_str().unwrap()
        ))
        .await["status"],
        "failed"
    );
}
#[tokio::test]
async fn worker_process_crash_restarts_and_identical_request_recovers() {
    recovery("crash", "WORKER_FAILED").await;
}
#[tokio::test]
async fn worker_timeout_kills_process_and_identical_request_recovers() {
    recovery("timeout", "JOB_TIMEOUT").await;
}
#[tokio::test]
async fn queue_and_pixel_caps_are_enforced() {
    let (_dir, workspace) = cli("hang");
    let s = Server::configured(|c| {
        c.workspace = workspace;
        c.job_timeout = Duration::from_secs(5);
    })
    .await;
    let p = s.project().await;
    s.batch(&p, 0, "initial", design()).await;
    let mut rejected = 0;
    for width in 321..334 {
        let result = render(&s, &p, width).await;
        if result["error"]["code"] == "QUEUE_FULL" {
            rejected += 1;
            assert_eq!(result["error"]["retryable"], true);
        }
    }
    assert!(rejected >= 4);
    let result=s.call("render_frame",json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":100000,"height":100000}})).await;
    assert_eq!(result["error"]["code"], "INVALID_REQUEST");
}
