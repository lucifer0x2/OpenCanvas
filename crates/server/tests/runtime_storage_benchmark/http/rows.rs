use serde_json::{json, Map, Value};

// An independent reader verifies the public wire format without sharing the production encoder.
pub fn unpack(mut snapshot: Value) -> Value {
    const CORE: [&str; 5] = ["id", "parentId", "sortKey", "kind", "props"];
    let object = snapshot.as_object_mut().unwrap();
    assert_eq!(object.remove("nodeFormat"), Some(json!("rows-v1")));
    assert_eq!(
        object.remove("nodeColumns"),
        Some(json!([
            "id", "parentId", "sortKey", "kind", "props", "extra"
        ]))
    );
    let nodes = object.get_mut("nodes").unwrap().as_array_mut().unwrap();
    for node in nodes {
        let Value::Array(mut row) = node.take() else {
            panic!("each rows-v1 node is an array");
        };
        assert!((CORE.len()..=CORE.len() + 1).contains(&row.len()));
        let mut fields = Map::new();
        if row.len() > CORE.len() {
            let Some(Value::Object(extra)) = row.pop() else {
                panic!("rows-v1 extras must be an object when present");
            };
            fields = extra;
        }
        for (column, value) in CORE.into_iter().zip(row) {
            assert!(fields.insert(column.to_owned(), value).is_none());
        }
        *node = Value::Object(fields);
    }
    snapshot
}
