mod library;
mod support;
use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

fn request(args: Value) -> ReadRequest {
    serde_json::from_value(args).unwrap()
}

fn equivalent(scoped: &ScopedSnapshot<'_>, args: Value) -> Value {
    let reference = legacy::read(scoped.snapshot(), request(args.clone()))
        .unwrap_or_else(|error| error.value());
    let actual = read_scoped(scoped, request(args.clone()))
        .map(|(value, _)| value)
        .unwrap_or_else(|error| error.value());
    assert_eq!(
        serde_json::to_vec(&actual).unwrap(),
        serde_json::to_vec(&reference).unwrap(),
        "{args}"
    );
    actual
}

#[test]
fn all_match_criteria_scopes_paths_and_unicode_pages_match_full_reference() {
    let snapshot = support::fixture(3);
    let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
    for scope in [
        json!({}),
        json!({"nodeId":"page2"}),
        json!({"frameId":"frame0"}),
        json!({"nodeId":"placed"}),
        json!({"frameId":"placed/nested"}),
        json!({"nodeId":"placed/nested/master-text"}),
    ] {
        for criterion in [
            json!({"nameContains":"needle"}),
            json!({"textContains":"needle"}),
            json!({"kinds":["instance","text"]}),
            json!({"ids":["deep-text","placed","placed/nested/master-text"]}),
            json!({"parentId":"frame2"}),
            json!({"nameContains":"needle","textContains":"NEEDLE","kinds":["text"]}),
            json!({"nameContains":"absent"}),
        ] {
            let mut args = json!({"projectId":snapshot.project_id,"limit":2,"budget":1300,"properties":["name"]});
            args.as_object_mut()
                .unwrap()
                .extend(scope.as_object().unwrap().clone());
            args.as_object_mut()
                .unwrap()
                .extend(criterion.as_object().unwrap().clone());
            let mut pages = 0;
            loop {
                let value = equivalent(&scoped, args.clone());
                assert!(value.get("error").is_none(), "{value}");
                pages += 1;
                if value["nextCursor"].is_null() {
                    break;
                }
                args["cursor"] = value["nextCursor"].clone();
                assert!(pages < 100);
            }
        }
    }
    let deep = equivalent(
        &scoped,
        json!({"projectId":snapshot.project_id,"ids":["deep-text"]}),
    );
    assert_eq!(deep["matches"][0]["pathTruncated"], true);
    let unicode = equivalent(
        &scoped,
        json!({"projectId":snapshot.project_id,"ids":["text0-0"],"textContains":"needle"}),
    );
    assert_eq!(
        unicode["matches"][0]["textPreview"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        120
    );
}

#[test]
fn exact_budget_boundaries_and_error_priority_match_reference() {
    let snapshot = support::fixture(2);
    let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
    for budget in [3, 100, 320, 512, 1024, 1300, 2048, 65536] {
        for limit in [0, 1, 3, 100] {
            equivalent(
                &scoped,
                json!({"projectId":snapshot.project_id,"textContains":"needle","properties":["text"],"budget":budget,"limit":limit}),
            );
        }
    }
    for args in [
        json!({"projectId":snapshot.project_id,"cursor":"bad","revision":0}),
        json!({"projectId":snapshot.project_id,"nodeId":"x","frameId":"x","kinds":["text"],"cursor":"bad"}),
        json!({"projectId":snapshot.project_id,"nodeId":"missing","kinds":["text"]}),
        json!({"projectId":snapshot.project_id,"nodeId":"placed/missing","kinds":["text"]}),
        json!({"projectId":snapshot.project_id,"frameId":"placed","kinds":["text"]}),
        json!({"projectId":snapshot.project_id,"frameId":"placed/nested/master-text","kinds":["text"]}),
        json!({"projectId":snapshot.project_id,"kinds":["text"],"cursor":"bad","revision":0}),
    ] {
        equivalent(&scoped, args);
    }
    let mut args = json!({"projectId":snapshot.project_id,"kinds":["text"],"limit":1});
    let first = equivalent(&scoped, args.clone());
    args["cursor"] = first["nextCursor"].clone();
    let mut decoded: Value = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(args["cursor"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    decoded["offset"] = json!(99999);
    args["cursor"] = json!(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&decoded).unwrap()));
    equivalent(&scoped, args);
}

#[test]
fn global_counts_visit_all_pages_but_expansion_and_matches_are_bounded() {
    let snapshot = support::fixture(12);
    let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
    let args = json!({"projectId":snapshot.project_id,"nameContains":"needle","limit":1});
    let (value, work) = read_scoped(&scoped, request(args.clone())).unwrap();
    equivalent(&scoped, args);
    assert_eq!(
        work.visited,
        value["scannedNodes"].as_u64().unwrap() as usize
    );
    assert!(work.visited > 150, "{work:?}");
    assert!(work.max_expanded < 40, "{work:?}");
    assert_eq!(work.materialized_matches, 1);
    assert_eq!(work.retained_matches, 1);
    eprintln!("global find limit=1: {work:?}; exact counts include all twelve pages, SQL/cold index work excluded");
    let args = json!({"projectId":snapshot.project_id,"textContains":"needle","budget":1024,"properties":["text"]});
    let (_, work) = read_scoped(&scoped, request(args.clone())).unwrap();
    equivalent(&scoped, args);
    assert!(work.materialized_matches <= 3, "{work:?}");
    eprintln!("global find budget=1024: {work:?}; SQL/cold index work excluded");
}

#[test]
fn untrusted_invalid_snapshots_keep_legacy_expand_only_behavior() {
    use opencanvas_storage::{SqliteBackend, StorageBackend, Store};
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let backend = SqliteBackend::new(Store::open(root.path()).unwrap());
    let project = backend.create_project("untrusted", "Untrusted").unwrap();
    let shared = backend.shared_snapshot(&project.project_id).unwrap();
    let mut snapshot = support::fixture(2);
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "frame0")
        .unwrap()
        .props["unknownProp"] = json!(true);
    assert!(ScopedSnapshot::validate(&snapshot).is_err());
    let untrusted = SharedSnapshot::untrusted(Arc::new(snapshot), shared.indexes);
    for args in [
        json!({"projectId":untrusted.snapshot.project_id,"kinds":["frame"]}),
        json!({"projectId":untrusted.snapshot.project_id}),
        json!({"projectId":untrusted.snapshot.project_id,"kinds":["frame"],"revision":0}),
    ] {
        let reference =
            legacy::read(&untrusted.snapshot, request(args.clone())).unwrap_or_else(|e| e.value());
        let actual = read_shared(&untrusted, request(args)).unwrap_or_else(|e| e.value());
        assert_eq!(
            serde_json::to_vec(&actual).unwrap(),
            serde_json::to_vec(&reference).unwrap()
        );
    }
}
