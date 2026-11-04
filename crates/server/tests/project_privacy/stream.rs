use super::*;
use reqwest::Response;

const VISIBILITY_EVENT: &str = "event: project_visibility_changed\ndata: {}\n\n";

async fn next_event(stream: &mut Response) -> Option<String> {
    tokio::time::timeout(Duration::from_secs(6), stream.chunk())
        .await
        .expect("SSE event or EOF")
        .unwrap()
        .map(|chunk| String::from_utf8(chunk.to_vec()).unwrap())
}

#[tokio::test]
async fn public_sse_filters_private_activity_and_closes_an_open_project_after_demotion() {
    let local = Server::start().await;
    let public = PublicServer::start(local.state.clone()).await;
    let shown = local.project().await;
    set_privacy(&local, &shown, "public").await;
    let mut workspace = public
        .request(Method::GET, "/api/v1/events")
        .send()
        .await
        .unwrap();
    let mut project = public
        .request(Method::GET, &format!("{PROJECTS}/{shown}/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(workspace.status(), 200);
    assert_eq!(project.status(), 200);
    let hidden = local
        .call(
            "create_project",
            json!({"slug":"hidden","name":"Hidden design"}),
        )
        .await;
    let hidden_id = hidden["projectId"].as_str().unwrap();
    let page = json!([{"op":"create_node","id":"page","kind":"page","parentId":null,"props":{"name":"Page"}}]);
    assert_eq!(
        local.batch(hidden_id, 0, "hidden", page.clone()).await["committedRevision"],
        1
    );
    assert_eq!(
        local.batch(&shown, 0, "shown", page).await["committedRevision"],
        1
    );
    let activity = next_event(&mut workspace).await.unwrap();
    assert!(activity.contains("event: project_activity"), "{activity}");
    assert!(activity.contains(&shown), "{activity}");
    assert!(!activity.contains(hidden_id), "{activity}");
    let replay = next_event(&mut project).await.unwrap();
    assert!(replay.contains("event: commit"), "{replay}");
    assert!(!replay.contains(hidden_id), "{replay}");

    set_privacy(&local, hidden_id, "public").await;
    assert_eq!(
        next_event(&mut workspace).await.as_deref(),
        Some(VISIBILITY_EVENT)
    );
    assert_eq!(
        next_event(&mut project).await.as_deref(),
        Some(VISIBILITY_EVENT)
    );
    set_privacy(&local, &shown, "private").await;
    let _ = local
        .state
        .events
        .send(json!({"event":"scene_ready","projectId":shown,"jobId":"hidden-job"}));
    assert_eq!(
        next_event(&mut workspace).await.as_deref(),
        Some(VISIBILITY_EVENT)
    );
    assert_eq!(
        next_event(&mut project).await.as_deref(),
        Some(VISIBILITY_EVENT)
    );
    assert!(next_event(&mut project).await.is_none());

    // Writes from a separate application instance do not reach this process's broadcaster.
    let mut other_project = public
        .request(Method::GET, &format!("{PROJECTS}/{hidden_id}/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(other_project.status(), 200);
    let db = rusqlite::Connection::open(local.root.path().join("hidden/db.sql")).unwrap();
    db.execute(
        "UPDATE project_meta SET privacy_status='private' WHERE project_id=?1",
        [hidden_id],
    )
    .unwrap();
    assert_eq!(
        next_event(&mut workspace).await.as_deref(),
        Some(VISIBILITY_EVENT)
    );
    assert_eq!(
        next_event(&mut other_project).await.as_deref(),
        Some(VISIBILITY_EVENT)
    );
    assert!(next_event(&mut other_project).await.is_none());
}
