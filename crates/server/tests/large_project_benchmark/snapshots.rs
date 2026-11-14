//! Item 1 (storage snapshot load, Moka hit) and item 2 (HTTP GET /snapshot).
use super::stats::{ms, sample, summary, Plan};
use super::Backends;
use flate2::{write::GzEncoder, Compression};
use opencanvas_server::{router, AppState, Config};
use serde_json::{json, Value};
use std::{io::Write, path::PathBuf, time::Instant};

/// Moka and the HTTP response cache both refuse entries above 16 MiB.
pub const CACHE_ENTRY_LIMIT: usize = 16 * 1024 * 1024;

/// Blocking: run on a blocking worker.
pub fn storage(plan: &Plan, backends: &Backends, project: &str) -> Value {
    let (cold, snapshot) = sample(plan, |_| backends.uncached.snapshot(project).unwrap());
    let json_bytes = serde_json::to_vec(&snapshot).unwrap().len();
    let (encode, _) = sample(plan, |_| {
        // Same steps as the object representation: DOM, envelope field, bytes.
        let mut value = serde_json::to_value(&snapshot).unwrap();
        value["truncated"] = json!(false);
        serde_json::to_vec(&value).unwrap().len()
    });
    let mut out = json!({
        "coldLoad": cold, "snapshotJsonBytes": json_bytes,
        "cacheAdmitted": json_bytes <= CACHE_ENTRY_LIMIT, "encodeObjects": encode,
    });
    if backends.moka {
        backends.primary.snapshot(project).unwrap(); // fill, if admitted
        let (warm, _) = sample(plan, |_| backends.primary.snapshot(project).unwrap());
        out["mokaWarm"] = warm;
    }
    out
}

pub async fn http(plan: &Plan, backends: &Backends, project: &str) -> Value {
    let mut out = json!({});
    for (label, query) in [("objects", ""), ("rows", "?nodeFormat=rows")] {
        let mut cold = Vec::new();
        let mut repeat = Vec::new();
        let mut body = Vec::new();
        for index in 0..plan.total() {
            // A fresh AppState has an empty response cache: its first GET encodes.
            let (base, task) = serve(backends).await;
            let url = format!("{base}/api/v1/projects/{project}/snapshot{query}");
            let client = reqwest::Client::new();
            let start = Instant::now();
            body = get(&client, &url).await;
            let first = ms(start);
            let start = Instant::now();
            let again = get(&client, &url).await;
            let second = ms(start);
            assert_eq!(again.len(), body.len());
            task.abort();
            if plan.measured(index) {
                cold.push(first);
                repeat.push(second);
            }
        }
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(&body).unwrap();
        let gzip_bytes = gzip.finish().unwrap().len();
        if label == "objects" {
            if let Ok(path) = std::env::var("TASK24_BODY_FILE") {
                std::fs::write(path, &body).unwrap();
            }
        }
        out[label] = json!({
            "bodyBytes": body.len(), "gzipBytes": gzip_bytes,
            "responseCacheAdmitted": body.len() <= CACHE_ENTRY_LIMIT,
            "getCold": summary(&cold), "getRepeat": summary(&repeat),
        });
    }
    out
}

/// A real listener and router over the primary storage, with fresh in-memory caches.
pub async fn serve(backends: &Backends) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = Config::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        listener.local_addr().unwrap().port(),
    );
    let base = config.base_url.clone();
    let app = router(AppState::new(backends.primary.clone(), config));
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (base, task)
}

async fn get(client: &reqwest::Client, url: &str) -> Vec<u8> {
    let response = client.get(url).send().await.unwrap();
    assert!(response.status().is_success(), "{}", response.status());
    response.bytes().await.unwrap().to_vec()
}
