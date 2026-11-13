//! Global find over many pages sharing one component library.
use super::{super::dispatch, equivalent, legacy, read_scoped, request};
use opencanvas_storage::{read_scope::*, BatchRequest, Snapshot, Storage, Store};
use serde_json::{json, Value};

/// `pages` pages each place one instance of a master with `library` children.
fn library_fixture(pages: usize, library: usize) -> (tempfile::TempDir, Storage, Snapshot) {
    let root = tempfile::tempdir().unwrap();
    let storage: Storage = Store::open(root.path()).unwrap().into();
    let project = storage.create_project("find-library", "Library").unwrap();
    let node = |id: &str, kind: &str, parent: Option<&str>, props: Value| json!({"op":"create_node","id":id,"kind":kind,"parentId":parent,"props":props});
    let mut ops = vec![
        node("library", "page", None, json!({"name":"Library"})),
        node(
            "master",
            "frame",
            Some("library"),
            json!({"name":"Needle master","component":{"name":"Card"},"width":100,"height":100}),
        ),
    ];
    for i in 0..library {
        let props = json!({"name":format!("Needle part {i}"),"width":5,"height":5});
        ops.push(node(
            &format!("part{i}"),
            "rectangle",
            Some("master"),
            props,
        ));
    }
    for p in 0..pages {
        let (page, frame) = (format!("page{p}"), format!("frame{p}"));
        ops.push(node(&page, "page", None, json!({"name":"Needle page"})));
        let props = json!({"name":"Needle frame","width":300,"height":300});
        ops.push(node(&frame, "frame", Some(&page), props));
        let props = json!({"name":"Needle card","componentId":"master"});
        ops.push(node(&format!("card{p}"), "instance", Some(&frame), props));
    }
    let request: BatchRequest = serde_json::from_value(json!({"projectId":project.id(),"expectedRevision":0,"idempotencyKey":"library","operations":ops})).unwrap();
    project.apply_batch(request).unwrap();
    let snapshot = project.snapshot().unwrap();
    (root, storage, snapshot)
}

#[test]
fn shared_library_work_grows_with_authored_nodes_not_pages_times_library() {
    let pages = 8;
    let mut selected = Vec::new();
    for library in [10, 40] {
        let (_root, _storage, snapshot) = library_fixture(pages, library);
        let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
        for args in [
            json!({"nameContains":"needle","limit":3}),
            json!({"kinds":["rectangle"],"budget":2048}),
            json!({"nameContains":"absent"}),
        ] {
            let mut args = args;
            args["projectId"] = json!(snapshot.project_id);
            equivalent(&scoped, args);
        }
        let args = json!({"projectId":snapshot.project_id,"nameContains":"needle"});
        let (value, work) = read_scoped(&scoped, request(args)).unwrap();
        assert_eq!(work.selected, snapshot.nodes.len(), "{work:?}");
        assert_eq!(
            value["scannedNodes"],
            snapshot.nodes.len() + pages * library
        );
        // The previous per-root dependency closure re-walked the library each page.
        let closure: usize = scoped
            .roots()
            .map(|root| {
                let nodes = scoped.expand(ReadScope::Target(&root.id), false).unwrap();
                nodes.stats.closure_nodes
            })
            .sum();
        eprintln!(
            "library={library}: selected={} legacy closure={closure}",
            work.selected
        );
        assert!(closure >= work.selected + pages * library);
        selected.push(work.selected);
    }
    assert_eq!(selected[1] - selected[0], 30);
}

#[test]
fn committed_sqlite_shared_snapshot_takes_the_scoped_find_path() {
    let (_root, storage, snapshot) = library_fixture(3, 5);
    let shared = storage.shared_snapshot(&snapshot.project_id).unwrap();
    for args in [
        json!({"nameContains":"needle","limit":2}),
        json!({"frameId":"frame1","kinds":["rectangle"]}),
        json!({"ids":["card2/part4","part0"]}),
    ] {
        let mut args = args;
        args["projectId"] = json!(snapshot.project_id);
        let (value, work) = dispatch(&shared, request(args.clone())).unwrap();
        let work = work.expect("builtin committed snapshot uses the scoped path");
        assert!(work.selected > 0 && work.visited > 0, "{work:?}");
        let reference = legacy::read(&shared.snapshot, request(args.clone())).unwrap();
        let bytes = |v: &Value| serde_json::to_vec(v).unwrap();
        assert_eq!(bytes(&value), bytes(&reference), "{args}");
    }
}
