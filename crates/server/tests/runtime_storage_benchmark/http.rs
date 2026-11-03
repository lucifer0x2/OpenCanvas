mod rows;

use super::{
    hot_proof,
    metrics::{elapsed_us, Samples, SAMPLES, WARMUP},
    storage::{self, Measured},
};
use axum::body::Bytes;
use opencanvas_server::{router, AppState, Config};
use reqwest::{
    header::{HeaderValue, ETAG, IF_NONE_MATCH},
    Client, StatusCode,
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub async fn measure(measured: &Measured, postgres: bool) -> Value {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = Config::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        listener.local_addr().unwrap().port(),
    );
    let url = format!(
        "{}/api/v1/projects/{}/snapshot",
        config.base_url,
        measured.project.handle.id()
    );
    let rows_url = format!("{url}?nodeFormat=rows");
    let state = AppState::new(measured.project.storage.clone(), config);
    let task = tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    // Fill both serialized representations before counting SQL or timing steady-state requests.
    let full = get(&client, &url, None).await;
    let rows = get(&client, &rows_url, None).await;
    assert_eq!(full.status, StatusCode::OK);
    assert_eq!(rows.status, StatusCode::OK);
    assert_ne!(full.etag, rows.etag);
    let full_value: Value = serde_json::from_slice(&full.body).unwrap();
    let rows_value: Value = serde_json::from_slice(&rows.body).unwrap();
    let mut expected = serde_json::to_value(&measured.snapshot).unwrap();
    expected["truncated"] = json!(false);
    assert_eq!(full_value, expected);
    assert_eq!(rows::unpack(rows_value), expected);
    for (url, own, other) in [(&url, &full, &rows), (&rows_url, &rows, &full)] {
        let response = get(&client, url, Some(&other.etag)).await;
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.etag, own.etag);
        assert_eq!(response.body, own.body);
    }

    let full = measure_format(&client, &url, full, postgres).await;
    let rows = measure_format(&client, &rows_url, rows, postgres).await;
    task.abort();
    let _ = task.await;
    let project = measured.project.handle.clone();
    let snapshot = measured.snapshot.clone();
    let count = measured.project.count;
    let seed_revision = measured.project.seed_revision;
    let storage = measured.project.storage.clone();
    tokio::task::spawn_blocking(move || {
        let project = super::fixture::Project {
            storage,
            handle: project,
            moka: false,
            count,
            seed_revision,
            _root: None,
        };
        assert_eq!(project.handle.snapshot().unwrap(), snapshot);
        storage::verify(&project, &snapshot);
    })
    .await
    .unwrap();
    json!({"full":full, "rows":rows})
}

struct Reply {
    status: StatusCode,
    etag: HeaderValue,
    body: Bytes,
}

async fn get(client: &Client, url: &str, validator: Option<&HeaderValue>) -> Reply {
    let mut request = client.get(url);
    if let Some(validator) = validator {
        request = request.header(IF_NONE_MATCH, validator);
    }
    let response = request.send().await.unwrap();
    Reply {
        status: response.status(),
        etag: response.headers()[ETAG].clone(),
        body: response.bytes().await.unwrap(),
    }
}

async fn measure_format(client: &Client, url: &str, expected: Reply, postgres: bool) -> Value {
    let mut receive = Vec::new();
    let mut parse = Vec::new();
    let mut total = Vec::new();
    let before = hot_proof::counters(postgres);
    for index in 0..WARMUP + SAMPLES {
        let start = Instant::now();
        let reply = get(client, url, None).await;
        let received = elapsed_us(start);
        let parse_start = Instant::now();
        let value: Value = serde_json::from_slice(&reply.body).unwrap();
        let parsed = elapsed_us(parse_start);
        let elapsed = elapsed_us(start);
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.etag, expected.etag);
        // The prefill bodies already passed exact semantic parity against the authored snapshot.
        assert_eq!(reply.body, expected.body);
        assert!(value.is_object());
        if index >= WARMUP {
            receive.push(received);
            parse.push(parsed);
            total.push(elapsed);
        }
    }
    let hot_proof = hot_proof::verify(before, WARMUP + SAMPLES);

    let mut not_modified = Vec::new();
    let before = hot_proof::counters(postgres);
    for index in 0..WARMUP + SAMPLES {
        let start = Instant::now();
        let reply = get(client, url, Some(&expected.etag)).await;
        let received = elapsed_us(start);
        assert_eq!(reply.status, StatusCode::NOT_MODIFIED);
        assert_eq!(reply.etag, expected.etag);
        assert!(reply.body.is_empty());
        if index >= WARMUP {
            not_modified.push(received);
        }
    }
    let conditional_hot_proof = hot_proof::verify(before, WARMUP + SAMPLES);
    json!({
        "receiveBody":Samples::new(receive), "parseJson":Samples::new(parse),
        "total":Samples::new(total), "responseBodyBytes":expected.body.len(), "hotProof":hot_proof,
        "notModified":{
            "receiveBody":Samples::new(not_modified), "responseBodyBytes":0,
            "hotProof":conditional_hot_proof,
        }
    })
}
