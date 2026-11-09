mod public_support;
#[allow(dead_code)]
mod support;
use opencanvas_server::RenderBackend;
use public_support::{set_privacy, PublicServer, ORIGIN};
use reqwest::Method;
use serde_json::{json, Value};
use support::Server;

async fn native() -> Server {
    Server::configured(|config| config.render_backend = RenderBackend::Rust).await
}

fn frames(count: usize) -> Value {
    let mut ops =
        vec![json!({"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}})];
    for index in 0..count {
        ops.push(
            json!({"op":"create_node","id":format!("f{index:02}"),"kind":"frame","parentId":"page",
            "props":{"width":100+index,"height":80+index,"fill":"#fff"}}),
        );
    }
    Value::Array(ops)
}

#[tokio::test]
async fn batches_reach_all_eleven_frames_at_authored_and_shared_sizes() {
    let server = native().await;
    let project = server.project().await;
    let mut ops = frames(11);
    ops.as_array_mut().unwrap().push(json!({"op":"create_node","id":"nested","kind":"frame","parentId":"f00","props":{"width":10,"height":10}}));
    assert_eq!(
        server.batch(&project, 0, "seed", ops).await["committedRevision"],
        1
    );
    let path = format!("/api/v1/projects/{project}/scenes?pageId=page&revision=1");
    let mut seen = Vec::new();
    let mut offset = 0;
    loop {
        let batch = server.get(&format!("{path}&offset={offset}")).await;
        assert_eq!(batch["total"], 11, "{batch}");
        assert_eq!(batch["projectId"], project);
        assert_eq!(batch["revision"], 1);
        for job in batch["jobs"].as_array().unwrap() {
            let ready = server.wait_job(job["jobId"].as_str().unwrap()).await;
            let index = seen.len();
            assert_eq!(ready["frameId"], format!("f{index:02}"));
            assert_eq!(ready["status"], "ready", "{ready}");
            assert_eq!(
                ready["scene"]["provenance"]["viewport"]["width"],
                json!(100.0 + index as f64)
            );
            assert_eq!(
                ready["scene"]["provenance"]["viewport"]["height"],
                json!(80.0 + index as f64)
            );
            seen.push(ready["frameId"].clone());
        }
        match batch["nextOffset"].as_u64() {
            Some(next) => {
                assert!(next > offset);
                offset = next;
            }
            None => break,
        }
    }
    assert_eq!(seen.len(), 11);
    let preset = server
        .get(&format!("{path}&offset=10&limit=1&width=320&height=240"))
        .await;
    let job = server
        .wait_job(preset["jobs"][0]["jobId"].as_str().unwrap())
        .await;
    assert_eq!(
        job["scene"]["provenance"]["viewport"],
        json!({"width":320.0,"height":240.0})
    );
    let end = server.get(&format!("{path}&offset=11")).await;
    assert_eq!(end["jobs"], json!([]));
    assert_eq!(end["nextOffset"], Value::Null);
}

#[tokio::test]
async fn empty_single_and_invalid_queries_are_explicit() {
    for count in [0, 1] {
        let server = native().await;
        let project = server.project().await;
        assert_eq!(
            server.batch(&project, 0, "seed", frames(count)).await["committedRevision"],
            1
        );
        let path = format!("/api/v1/projects/{project}/scenes?pageId=page&revision=1");
        let batch = server.get(&path).await;
        assert_eq!(batch["jobs"].as_array().unwrap().len(), count, "{batch}");
        assert_eq!(batch["nextOffset"], Value::Null);
        for suffix in [
            "&limit=0",
            "&limit=5",
            "&offset=99",
            "&width=320",
            "&width=0&height=1",
            "&unknown=x",
            "&offset=-1",
            "&revision=2",
        ] {
            let response = server
                .client
                .get(format!("{}{path}{suffix}", server.url))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 400, "{suffix}");
            assert_eq!(
                response.json::<Value>().await.unwrap()["error"]["code"],
                "INVALID_REQUEST"
            );
        }
        let wrong = server
            .get(&format!(
                "/api/v1/projects/{project}/scenes?pageId=page&revision=0"
            ))
            .await;
        assert_eq!(wrong["error"]["code"], "REVISION_UNAVAILABLE");
        let missing = server
            .get(&format!(
                "/api/v1/projects/{project}/scenes?pageId=f00&revision=1"
            ))
            .await;
        assert_eq!(missing["error"]["code"], "INVALID_REQUEST");
    }
}

#[tokio::test]
async fn public_batches_apply_auth_privacy_and_project_only_resource_urls() {
    let local = native().await;
    let public = PublicServer::start(local.state.clone()).await;
    let project = local.project().await;
    let mut ops = frames(1);
    ops.as_array_mut().unwrap().push(json!({"op":"create_node","id":"text","kind":"text","parentId":"f00",
        "props":{"text":"Public","width":90,"height":30,"fontId":"font_bundled","fontSize":12,"lineHeight":16}}));
    assert_eq!(
        local.batch(&project, 0, "seed", ops).await["committedRevision"],
        1
    );
    let path = format!("/api/v1/projects/{project}/scenes?pageId=page&revision=1");
    let denied = public.request(Method::GET, &path).send().await.unwrap();
    assert_eq!(denied.status(), 404);
    set_privacy(&local, &project, "public").await;
    let initial = public.get(&path).await;
    let id = initial["jobs"][0]["jobId"].as_str().unwrap();
    let ready = local.wait_job(id).await;
    assert_eq!(ready["status"], "ready", "{ready}");
    let cached = public.get(&path).await;
    let job = &cached["jobs"][0];
    assert_eq!(job["jobId"], id);
    assert!(job["imageUrl"].as_str().unwrap().starts_with(ORIGIN));
    assert!(job["scene"]["fonts"][0]["url"]
        .as_str()
        .unwrap()
        .starts_with(ORIGIN));
    assert!(ready["imageUrl"].as_str().unwrap().starts_with(&local.url));
    assert_eq!(
        public.resource(&job["imageUrl"]).await.headers()["content-type"],
        "image/png"
    );
    assert_eq!(
        public
            .resource(&job["scene"]["fonts"][0]["url"])
            .await
            .headers()["content-type"],
        "font/ttf"
    );
    set_privacy(&local, &project, "private").await;
    assert_eq!(
        public
            .request(Method::GET, &path)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        public
            .request(Method::GET, &format!("/api/v1/jobs/{id}"))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    local
        .call(
            "set_public_viewer_password",
            json!({"password":"batch-test-password"}),
        )
        .await;
    assert_eq!(
        public
            .request(Method::GET, &path)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
}
