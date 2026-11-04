use opencanvas_storage::{
    PostgresBackend, Storage, Store, ViewerAuthConfig, ViewerAuthStore, ViewerSession,
};

const IP: &str = "203.0.113.5";
const DIFFERENT_IP: &str = "203.0.113.6";
const FIRST_HASH: &str = "first-test-hash";
const SECOND_HASH: &str = "second-test-hash";

fn credentials(hash: &str) -> Option<ViewerAuthConfig> {
    Some(ViewerAuthConfig {
        password_hash: hash.into(),
        signing_key: vec![7; 32],
    })
}

fn issue(auth: &dyn ViewerAuthStore, hash: &str, id: &str, expires: i64) -> bool {
    auth.issue(
        hash.into(),
        ViewerSession {
            session_id: id.into(),
            ip_address: IP.into(),
            user_agent: "Test device".into(),
            issued_at: 100,
            expires_at: expires,
        },
        vec![1; 32],
    )
    .unwrap()
}

fn check(auth: &dyn ViewerAuthStore, id: &str, ip: &str, now: i64) -> bool {
    auth.check(id.into(), vec![1; 32], ip.into(), now).unwrap()
}

fn write_initial(store: Storage) {
    let auth = store.viewer_auth().unwrap();
    assert!(auth.config().unwrap().is_none());
    assert!(auth.list(100).unwrap().is_empty());
    assert!(!issue(auth, FIRST_HASH, "before-enable", 120));
    assert_eq!(auth.replace(credentials(FIRST_HASH)).unwrap(), 0);
    assert!(issue(auth, FIRST_HASH, "session", 120));
    assert!(issue(auth, FIRST_HASH, "expires", 105));
}

fn check_reopen(store: Storage) {
    let auth = store.viewer_auth().unwrap();
    let config = auth.config().unwrap().unwrap();
    assert_eq!(config.password_hash, FIRST_HASH);
    assert_eq!(config.signing_key, vec![7; 32]);
    let list = auth.list(105).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].session_id, "session");
    assert_eq!(list[0].user_agent, "Test device");
    assert!(!check(auth, "expires", IP, 105));
    assert!(!auth
        .check("session".into(), vec![2; 32], DIFFERENT_IP.into(), 105)
        .unwrap());
    assert!(check(auth, "session", IP, 105));
    assert!(!check(auth, "session", DIFFERENT_IP, 105));
    assert!(!check(auth, "session", IP, 105));
    assert!(issue(auth, FIRST_HASH, "old", 120));
    assert!(auth.replace(credentials(SECOND_HASH)).unwrap() >= 1);
    assert!(!issue(auth, FIRST_HASH, "late", 120));
    assert!(!check(auth, "old", IP, 105));
    assert!(issue(auth, SECOND_HASH, "new", 120));
    assert!(auth.revoke("new".into()).unwrap());
    assert!(!auth.revoke("new".into()).unwrap());
    assert!(issue(auth, SECOND_HASH, "disable", 120));
    assert_eq!(auth.replace(None).unwrap(), 1);
    assert!(auth.config().unwrap().is_none());
    assert!(auth.list(105).unwrap().is_empty());
    assert!(!issue(auth, SECOND_HASH, "late", 120));
}

#[test]
fn sqlite_viewer_sessions_are_durable_and_fail_closed_after_revocation() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path()).unwrap();
    assert!(store.list_projects().unwrap().is_empty());
    write_initial(store.into());
    let store = Store::open(root.path()).unwrap();
    assert!(store.list_projects().unwrap().is_empty());
    check_reopen(store.into());
}

#[test]
#[ignore = "requires a fresh isolated PostgreSQL database via OPENCANVAS_TEST_POSTGRES_URL"]
fn postgres_viewer_sessions_are_durable_and_fail_closed_after_revocation() {
    let dsn = std::env::var("OPENCANVAS_TEST_POSTGRES_URL").unwrap();
    write_initial(Storage::new(PostgresBackend::connect(&dsn).unwrap()));
    check_reopen(Storage::new(PostgresBackend::connect(&dsn).unwrap()));
}
