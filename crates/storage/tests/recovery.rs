#![cfg(feature = "fault-injection")]
mod common;
use common::*;
use opencanvas_storage::*;
use std::process::Command;

#[test]
fn crash_child() {
    let Ok(root) = std::env::var("OPENCANVAS_RECOVERY_ROOT") else {
        return;
    };
    let store = Store::open(root).unwrap();
    let p = store.open_project("fixture").unwrap();
    if std::env::var("OPENCANVAS_RECOVERY_ASSET").is_ok() {
        p.import_asset(asset_request(&p, 0, "lost-response"))
            .unwrap();
    } else {
        p.apply_batch(request(&p, 0, "lost-response", initial()))
            .unwrap();
    }
    panic!("Failpoint was not reached");
}

fn crash(root: &std::path::Path, point: &str, asset: bool) {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "crash_child", "--nocapture"])
        .env("OPENCANVAS_RECOVERY_ROOT", root)
        .env("OPENCANVAS_STORAGE_FAILPOINT", point);
    if asset {
        cmd.env("OPENCANVAS_RECOVERY_ASSET", "1");
    }
    assert_eq!(cmd.status().unwrap().code(), Some(91));
}

#[test]
fn abrupt_exit_before_and_after_commit_and_retry_lost_response() {
    for point in ["before_commit", "after_commit"] {
        let (temp, store, p) = setup();
        drop(p);
        drop(store);
        crash(temp.path(), point, false);
        let store = Store::open(temp.path()).unwrap();
        let p = store.open_project("fixture").unwrap();
        let committed = point == "after_commit";
        let s = p.snapshot().unwrap();
        assert_eq!(s.revision, u64::from(committed));
        assert_eq!(s.nodes.len(), if committed { 4 } else { 0 });
        let retry = p
            .apply_batch(request(&p, 0, "lost-response", initial()))
            .unwrap();
        assert_eq!(retry.replayed, committed);
        assert_eq!(retry.committed_revision, 1);
        assert_eq!(p.snapshot().unwrap().nodes.len(), 4);
        assert_eq!(p.commits_after(0, 100).unwrap().len(), 1);
    }
}

#[test]
fn asset_publish_crashes_leave_only_orphans_or_durable_committed_refs() {
    for point in ["after_asset_publish", "before_commit", "after_commit"] {
        let (temp, store, p) = setup();
        drop(p);
        drop(store);
        crash(temp.path(), point, true);
        let store = Store::open(temp.path()).unwrap();
        let p = store.open_project("fixture").unwrap();
        let committed = point == "after_commit";
        let s = p.snapshot().unwrap();
        assert_eq!(s.revision, u64::from(committed));
        assert_eq!(s.assets.len(), usize::from(committed));
        if committed {
            assert_eq!(p.read_asset(&s.assets[0].id).unwrap().1, png());
            assert!(p.orphan_asset_keys().unwrap().is_empty());
        } else {
            assert_eq!(p.orphan_asset_keys().unwrap().len(), 1);
        }
        let retry = p
            .import_asset(asset_request(&p, 0, "lost-response"))
            .unwrap();
        assert_eq!(retry.result.committed_revision, 1);
        assert_eq!(retry.result.replayed, committed);
        assert!(p.orphan_asset_keys().unwrap().is_empty());
    }
}
