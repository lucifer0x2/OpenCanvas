#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use support::Server;

#[tokio::test]
async fn viewer_exports_share_mcp_bounds_and_origin_guards_without_mutation() {
    let s = Server::start().await;
    let p = s.project().await;
    let url = format!("{}/api/v1/projects/{p}/exports", s.url);
    let invalid = [
        json!({"nodeIds":[]}),
        json!({"nodeIds":vec!["a";9]}),
        json!({"nodeIds":["a"],"waitMs":10001}),
        json!({"nodeIds":["a"],"settings":[{"format":"jpg"}]}),
        json!({"nodeIds":["a"],"settings":[{"scale":1,"width":100}]}),
        json!({"nodeIds":["a"],"unknown":true}),
        json!({"projectId":p,"nodeIds":["a"]}),
    ];
    for body in invalid {
        let response = s.client.post(&url).json(&body).send().await.unwrap();
        assert_eq!(response.status(), 400, "{body}");
    }
    let response = s
        .client
        .post(&url)
        .header("Origin", "https://evil.example")
        .json(&json!({"nodeIds":["a"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    assert_eq!(
        response.json::<Value>().await.unwrap()["error"]["code"],
        "ORIGIN_REJECTED"
    );
    let response = s
        .client
        .post(&url)
        .header("Host", "evil.example")
        .json(&json!({"nodeIds":["a"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    let response = s
        .client
        .post(&url)
        .header("Origin", &s.url)
        .json(&json!({"nodeIds":["missing"],"waitMs":0}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let body = response.json::<Value>().await.unwrap();
    assert_eq!(body["exports"][0]["error"]["code"], "NODE_NOT_FOUND");
    assert_eq!(
        s.get(&format!("/api/v1/projects/{p}/snapshot")).await["revision"],
        0
    );
}
