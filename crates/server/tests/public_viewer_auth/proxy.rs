use super::*;

#[tokio::test]
async fn proxy_replay_and_local_revocation_close_sessions() {
    let local = Server::start().await;
    let public = PublicServer::start_with(
        local.state.clone(),
        PublicViewer::new(ORIGIN)
            .unwrap()
            .with_trusted_proxy("127.0.0.1".parse().unwrap()),
    )
    .await;
    local.call(SET, json!({"password":PASSWORD})).await;
    const IP: &str = "203.0.113.1";
    const OTHER: &str = "203.0.113.2";
    assert_eq!(
        login(&public, PASSWORD)
            .header("forwarded", format!("for={IP}"))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    let first = cookie(
        login(&public, PASSWORD)
            .header("x-forwarded-for", IP)
            .send()
            .await
            .unwrap(),
    )
    .await;
    let authorized = |token: &str| {
        public
            .request(Method::GET, PROJECTS)
            .header("cookie", token)
    };
    for bad in ["garbage", "203.0.113.2, 203.0.113.1"] {
        assert_eq!(
            authorized(&first)
                .header("x-forwarded-for", bad)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
    }
    assert_eq!(
        authorized(&first)
            .header("x-forwarded-for", OTHER)
            .header("x-forwarded-for", IP)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        authorized(&format!("{first}A"))
            .header("x-forwarded-for", OTHER)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        authorized(&first)
            .header("x-forwarded-for", IP)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    let list = local.call(LIST, json!({})).await;
    assert_eq!(list["sessions"][0]["ipAddress"], IP);
    assert_eq!(
        list["sessions"][0]["expiresAt"].as_i64().unwrap()
            - list["sessions"][0]["issuedAt"].as_i64().unwrap(),
        7_776_000
    );
    assert_eq!(list["sessions"][0].as_object().unwrap().len(), 5);
    assert_eq!(
        authorized(&first)
            .header("x-forwarded-for", OTHER)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        authorized(&first)
            .header("x-forwarded-for", IP)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(local.call(LIST, json!({})).await, json!({"sessions":[]}));

    let second = cookie(
        login(&public, PASSWORD)
            .header("x-forwarded-for", IP)
            .send()
            .await
            .unwrap(),
    )
    .await;
    let id = local.call(LIST, json!({})).await["sessions"][0]["sessionId"].clone();
    let project = local.project().await;
    public_support::set_privacy(&local, &project, "public").await;
    let mut stream = public
        .request(Method::GET, &format!("{PROJECTS}/{project}/events"))
        .header("cookie", &second)
        .header("x-forwarded-for", IP)
        .send()
        .await
        .unwrap();
    assert_eq!(stream.status(), 200);
    assert_eq!(
        local.call(REVOKE, json!({"sessionId":id})).await,
        json!({"revoked":true})
    );
    assert!(tokio::time::timeout(Duration::from_secs(7), stream.chunk())
        .await
        .unwrap()
        .unwrap()
        .is_none());
    assert_eq!(
        authorized(&second)
            .header("x-forwarded-for", IP)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
}

#[tokio::test]
async fn enabling_password_closes_open_anonymous_sse_and_rotation_revokes_cookies() {
    const EVENTS: &str = "/api/v1/events";
    const NEW_PASSWORD: &str = "the rotated test passphrase";
    let local = Server::start().await;
    let public = PublicServer::start(local.state.clone()).await;
    let mut open = public.request(Method::GET, EVENTS).send().await.unwrap();
    assert_eq!(open.status(), 200);
    let project = local.project().await;
    public_support::set_privacy(&local, &project, "public").await;
    let message = tokio::time::timeout(Duration::from_secs(5), open.chunk())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&message),
        "event: project_visibility_changed\ndata: {}\n\n"
    );
    local.call(SET, json!({"password":PASSWORD})).await;
    assert!(tokio::time::timeout(Duration::from_secs(7), open.chunk())
        .await
        .unwrap()
        .unwrap()
        .is_none());
    let old = cookie(login(&public, PASSWORD).send().await.unwrap()).await;
    assert_eq!(
        local.call(SET, json!({"password":NEW_PASSWORD})).await,
        json!({"required":true,"revokedSessions":1})
    );
    assert_eq!(
        public
            .request(Method::GET, PROJECTS)
            .header("cookie", old)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(login(&public, PASSWORD).send().await.unwrap().status(), 401);
    let current = cookie(login(&public, NEW_PASSWORD).send().await.unwrap()).await;
    assert_eq!(
        public
            .request(Method::GET, PROJECTS)
            .header("cookie", current)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    assert_eq!(
        local.call(SET, json!({"password":null})).await,
        json!({"required":false,"revokedSessions":1})
    );
    assert_eq!(
        public.get(AUTH).await,
        json!({"authenticated":true,"required":false})
    );
    assert_eq!(local.call(LIST, json!({})).await, json!({"sessions":[]}));
    let disabled = login(&public, NEW_PASSWORD).send().await.unwrap();
    assert_eq!(disabled.status(), 200);
    assert!(disabled.headers().get("set-cookie").is_none());
    assert_eq!(
        disabled.json::<Value>().await.unwrap(),
        json!({"authenticated":true,"required":false})
    );
}
