use super::{ProjectPins, MAX_KEYS};
use crate::Store;
use std::sync::Arc;

#[test]
fn warm_ids_and_slugs_share_a_writer_and_cache_eviction_is_bounded() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path()).unwrap();
    let pins = ProjectPins::default();
    let first = store.create_project("first", "First").unwrap();
    let id = first.id().to_owned();
    let weak = Arc::downgrade(&first);
    pins.pin("first", &first);
    assert!(Arc::ptr_eq(&first, &pins.open(&store, "first").unwrap()));
    assert!(Arc::ptr_eq(&first, &pins.open(&store, &id).unwrap()));
    drop(first);
    assert!(weak.upgrade().is_some(), "adapter must retain warm writers");

    for index in 0..MAX_KEYS / 2 {
        let slug = format!("eviction-{index}");
        let project = store.create_project(&slug, "Eviction").unwrap();
        pins.pin(&slug, &project);
        assert!(pins.0.lock().unwrap().len() <= MAX_KEYS);
    }
    assert!(
        weak.upgrade().is_none(),
        "eviction must release old writers"
    );
    assert_eq!(pins.open(&store, &id).unwrap().id(), id);
    assert_eq!(pins.metadata(&store, "first").unwrap().project_id, id);
}
