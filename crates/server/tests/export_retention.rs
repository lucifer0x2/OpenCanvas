//! Export download URLs outlive render churn, expire after their retention or byte budget with
//! `EXPORT_EXPIRED` (410), and an unknown job tells the caller to re-run `export_nodes`.
#[allow(dead_code)]
mod export_support;
#[allow(dead_code)]
mod native_support;
use export_support::design;
use native_support::Server;
use opencanvas_server::RenderBackend;
use serde_json::{json, Value};
use std::time::Duration;

async fn export(s: &Server, p: &str, node: &str) -> Value {
    let args = json!({"projectId":p,"nodeIds":[node],"settings":[{"scale":1}]});
    let result = s.call("export_nodes", args).await;
    let item = result["exports"][0].clone();
    assert_eq!(item["status"], "ready", "{result}");
    assert!(item["expiresAt"].as_u64().is_some(), "{item}");
    item
}

async fn fetch(s: &Server, url: &str) -> (u16, reqwest::Response) {
    let response = s.client.get(url).send().await.unwrap();
    (response.status().as_u16(), response)
}

async fn error(s: &Server, url: &str, status: u16, code: &str) {
    let (got, response) = fetch(s, url).await;
    assert_eq!(got, status, "{url}");
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], code, "{body}");
    let message = body["error"]["message"].as_str().unwrap();
    assert!(message.contains("re-run export_nodes"), "{message}");
}

#[tokio::test]
async fn export_url_still_downloads_after_forty_other_jobs() {
    let s = Server::start().await;
    let p = s.project().await;
    assert_eq!(s.batch(&p, 0, design()).await["committedRevision"], 1);
    let item = export(&s, &p, "box").await;
    let file = &item["files"][0];
    let job = item["jobId"].as_str().unwrap();
    for width in 100..142 {
        let args = json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":width,"height":200}});
        let pending = s.call("render_frame", args).await;
        let done = s.wait(pending["jobId"].as_str().unwrap()).await;
        assert_eq!(done["status"], "ready", "{done}");
    }
    // The job record left the 32-record table; its files did not.
    let record = s.get(&format!("/api/v1/jobs/{job}")).await;
    assert_eq!(record["error"]["code"], "JOB_NOT_FOUND", "{record}");
    let (status, response) = fetch(&s, file["url"].as_str().unwrap()).await;
    assert_eq!(status, 200);
    assert_eq!(response.headers()["content-type"], "image/png");
    let bytes = response.bytes().await.unwrap();
    assert_eq!(bytes.len() as u64, file["bytes"].as_u64().unwrap());
    let wrong = format!("{}/api/v1/jobs/{job}/files/9", s.url);
    let (status, _) = fetch(&s, &wrong).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn expired_url_answers_gone_and_a_new_call_makes_a_new_job() {
    let s = Server::configured(RenderBackend::Rust, |config| {
        config.export_files.ttl = Duration::from_secs(4);
    })
    .await;
    let p = s.project().await;
    s.batch(&p, 0, design()).await;
    let item = export(&s, &p, "box").await;
    let url = item["files"][0]["url"].as_str().unwrap();
    assert_eq!(fetch(&s, url).await.0, 200);
    tokio::time::sleep(Duration::from_millis(4500)).await;
    error(&s, url, 410, "EXPORT_EXPIRED").await;
    // The record that listed the dead URL is gone too, so the identical call is a new job.
    let again = export(&s, &p, "box").await;
    assert_ne!(again["jobId"], item["jobId"]);
    assert_eq!(
        fetch(&s, again["files"][0]["url"].as_str().unwrap())
            .await
            .0,
        200
    );
    let unknown = format!("{}/api/v1/jobs/job_unknown/files/0", s.url);
    error(&s, &unknown, 404, "JOB_NOT_FOUND").await;
}

#[tokio::test]
async fn byte_budget_evicts_the_oldest_export_first() {
    let s = Server::configured(RenderBackend::Rust, |config| {
        config.export_files.max_bytes = 1;
    })
    .await;
    let p = s.project().await;
    s.batch(&p, 0, design()).await;
    let first = export(&s, &p, "box").await;
    let second = export(&s, &p, "inner").await;
    error(
        &s,
        first["files"][0]["url"].as_str().unwrap(),
        410,
        "EXPORT_EXPIRED",
    )
    .await;
    assert_eq!(
        fetch(&s, second["files"][0]["url"].as_str().unwrap())
            .await
            .0,
        200
    );
}
