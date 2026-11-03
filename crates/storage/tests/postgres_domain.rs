mod postgres_support;
#[path = "schemes/mod.rs"]
mod schemes;
use opencanvas_storage::*;
use postgres_support::*;
use serde_json::json;

fn same(pg: &ProjectHandle, sqlite: &ProjectHandle) {
    assert_eq!(
        normalized(pg.snapshot().unwrap()),
        normalized(sqlite.snapshot().unwrap())
    );
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn product_themes_instances_notes_order_and_cascade_match_sqlite() {
    let (_, storage) = setup();
    let pg = project(&storage, "domain");
    let root = tempfile::tempdir().unwrap();
    let sqlite: Storage = Store::open(root.path()).unwrap().into();
    let sq = sqlite.create_project("sqlite", "SQLite").unwrap();
    let mut seed = schemes::fixture();
    seed.extend(serde_json::from_value::<Vec<Operation>>(json!([
        {"op":"create_note","id":"note","nodeId":"box","text":"Tiếng Việt\0intent","author":"author"},
        {"op":"reply_note","id":"note","text":"Reply\0text","author":"reviewer"},
        {"op":"update_note","id":"note","resolved":true},
        {"op":"update_node","id":"label","props":{"text":"Text\0content"}}
    ])).unwrap());
    for p in [&pg, &sq] {
        p.apply_batch(request(p.id(), 0, "seed", seed.clone()))
            .unwrap();
    }
    same(&pg, &sq);
    let authored = pg.snapshot().unwrap();
    let expanded = resolved_expanded_nodes(&authored, "night").unwrap();
    let one = expanded.iter().find(|n| n.id == "one").unwrap();
    assert_eq!(one.props["fill"], "#abcdef");
    assert_eq!(one.props["effectiveTheme"], "night");
    assert_eq!(authored.notes[0].text, "Tiếng Việt\0intent");
    assert_eq!(pg.snapshot().unwrap(), authored);
    let mutate: Vec<Operation> = serde_json::from_value(json!([
        {"op":"update_note","id":"note","resolved":false,"nodeId":"frame","text":"Reanchored"},
        {"op":"reparent_node","id":"inner","parentId":"frame","index":0},
        {"op":"reorder_node","id":"two","index":0},
        {"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":{"default":"#000000","themes":{"night":"#ffffff","day":"#008800"}}}}
    ])).unwrap();
    for p in [&pg, &sq] {
        p.apply_batch(request(p.id(), 1, "mutate", mutate.clone()))
            .unwrap();
    }
    same(&pg, &sq);
    let delete = vec![Operation::DeleteSubtree { id: "frame".into() }];
    for p in [&pg, &sq] {
        let result = p
            .apply_batch(request(p.id(), 2, "cascade", delete.clone()))
            .unwrap();
        assert!(result.changed_ids.contains(&"note".into()));
    }
    same(&pg, &sq);
    assert!(pg.snapshot().unwrap().notes.is_empty());
    assert_eq!(pg.commits_after(0, 10).unwrap().len(), 3);
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn domain_failures_preserve_sqlite_code_field_index_and_full_rollback() {
    let (_, storage) = setup();
    let pg = project(&storage, "invalid");
    let root = tempfile::tempdir().unwrap();
    let sqlite: Storage = Store::open(root.path()).unwrap().into();
    let sq = sqlite.create_project("sqlite", "SQLite").unwrap();
    for p in [&pg, &sq] {
        p.apply_batch(request(p.id(), 0, "seed", schemes::fixture()))
            .unwrap();
    }
    let cases = json!([
        {"op":"update_node","id":"one","props":{"theme":"absent"}},
        {"op":"update_node","id":"one","props":{"overrides":{"box":{"radius":-1}}}},
        {"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":{"aliasId":"alias"}}},
        {"op":"create_note","id":"invalid-note","nodeId":"absent","text":"No target"},
        {"op":"update_node","id":"bad\0id","props":{"name":"Invalid target"}},
        {"op":"delete_subtree","id":"master"}
    ]);
    for (i, value) in cases.as_array().unwrap().iter().enumerate() {
        let op: Operation = serde_json::from_value(value.clone()).unwrap();
        let ops = vec![
            Operation::UpdateNode {
                id: "label".into(),
                props: json!({"text":"Must rollback"}),
            },
            op,
        ];
        let a = pg
            .apply_batch(request(pg.id(), 1, &format!("bad-{i}"), ops.clone()))
            .unwrap_err();
        let b = sq
            .apply_batch(request(sq.id(), 1, &format!("bad-{i}"), ops))
            .unwrap_err();
        assert_eq!(
            (&a.code, &a.field, a.op_index),
            (&b.code, &b.field, b.op_index),
            "{i}"
        );
        same(&pg, &sq);
        assert_eq!(pg.snapshot().unwrap().revision, 1);
    }
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn escaped_nul_project_name_and_large_json_numbers_survive_reconnect() {
    let (_, storage) = setup();
    let slug = format!("nul-{}", uuid::Uuid::new_v4().simple());
    let p = storage.create_project(&slug, "Name\0suffix").unwrap();
    let values = [json!(100_000_u64), json!(-100_000_i64), json!(0.0)];
    let ops = values
        .iter()
        .enumerate()
        .map(|(i, value)| Operation::UpsertToken {
            token: Token {
                id: format!("number-{i}"),
                name: format!("Number\0{i}"),
                token_type: "number".into(),
                value: value.clone(),
            },
        })
        .collect();
    p.apply_batch(request(p.id(), 0, "numbers", ops)).unwrap();
    let (_, reconnected) = setup();
    let snapshot = reconnected
        .open_project(p.id())
        .unwrap()
        .snapshot()
        .unwrap();
    assert_eq!(snapshot.name, "Name\0suffix");
    for (i, value) in values.iter().enumerate() {
        assert_eq!(
            &snapshot
                .tokens
                .iter()
                .find(|t| t.id == format!("number-{i}"))
                .unwrap()
                .value,
            value
        );
    }
}
