//! Task 07F item 1: the server pins opened projects, so reads by id or slug hit storage's
//! registry. A project directory removed or replaced out of band must still read exactly as it
//! did without pins: `PROJECT_NOT_FOUND` for the old identity, never another project's data.
mod support;
use serde_json::{json, Value};
use support::{design, Server};

async fn outline(s: &Server, project: &str) -> Value {
    s.call("get_outline", json!({"projectId":project})).await
}

#[tokio::test]
async fn pinned_projects_read_by_id_and_slug_and_never_outlive_their_directory() {
    let s = Server::start().await;
    let old = s.project().await;
    assert_eq!(
        s.batch(&old, 0, "seed", design()).await["committedRevision"],
        1
    );
    for key in [old.as_str(), "test-project"] {
        let read = outline(&s, key).await;
        assert_eq!(read["projectId"], old.as_str(), "{read}");
        assert_eq!(read["revision"], 1, "{read}");
    }
    let other = s
        .call("create_project", json!({"slug":"second","name":"Second"}))
        .await["projectId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(outline(&s, &other).await["projectId"], other.as_str());
    assert_eq!(outline(&s, &old).await["revision"], 1);

    // Replaced out of band: same slug, new identity. The pinned old handle now points at a
    // directory holding another project's database.
    std::fs::remove_dir_all(s.root.path().join("test-project")).unwrap();
    let new = s
        .call(
            "create_project",
            json!({"slug":"test-project","name":"Again"}),
        )
        .await["projectId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(new, old);
    let stale = outline(&s, &old).await;
    assert_eq!(stale["error"]["code"], "PROJECT_NOT_FOUND", "{stale}");
    let write = s
        .batch(
            &old,
            1,
            "late",
            json!([{"op":"update_node","id":"text","props":{"text":"x"}}]),
        )
        .await;
    assert_eq!(write["error"]["code"], "PROJECT_NOT_FOUND", "{write}");
    for key in [new.as_str(), "test-project"] {
        let read = outline(&s, key).await;
        assert_eq!(read["projectId"], new.as_str(), "{read}");
        assert_eq!(read["revision"], 0, "{read}");
    }

    // Removed out of band: reads report what an unpinned open reports.
    std::fs::remove_dir_all(s.root.path().join("second")).unwrap();
    for key in [other.as_str(), "second"] {
        let gone = outline(&s, key).await;
        assert_eq!(gone["error"]["code"], "PROJECT_NOT_FOUND", "{gone}");
    }
    assert_eq!(outline(&s, &new).await["projectId"], new.as_str());
}
