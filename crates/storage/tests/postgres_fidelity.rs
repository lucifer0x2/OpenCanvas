mod postgres_support;
use opencanvas_storage::*;
use postgres_support::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn versions(id: &str) -> BTreeMap<(String, String), String> {
    sql()
        .query(
            "SELECT kind,id,xmin::text FROM opencanvas_runtime_v1.entities WHERE project_id=$1",
            &[&id],
        )
        .unwrap()
        .into_iter()
        .map(|r| ((r.get(0), r.get(1)), r.get(2)))
        .collect()
}

fn zero_operations(value: Value) -> Vec<Operation> {
    vec![
        Operation::UpsertToken {
            token: Token {
                id: "zero".into(),
                name: "Zero".into(),
                token_type: "number".into(),
                value: value.clone(),
            },
        },
        Operation::UpdateNode {
            id: "box".into(),
            props: json!({"shadow":[{"x":value,"color":"#000000"}]}),
        },
    ]
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn signed_zero_changes_survive_commit_history_replay_and_reconnect_without_extra_writes() {
    let (_, storage) = setup();
    let p = project(&storage, "signed-zero");
    let mut seed = page("Signed zero");
    seed.extend(serde_json::from_value::<Vec<Operation>>(json!([
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":100,"height":100}},
        {"op":"create_node","id":"box","kind":"rectangle","parentId":"frame","props":{"width":10,"height":10}}
    ])).unwrap());
    seed.extend(zero_operations(json!(-0.0)));
    p.apply_batch(request(p.id(), 0, "seed", seed)).unwrap();
    let mut before = versions(p.id());
    for (revision, value) in [(1, json!(0.0)), (2, json!(-0.0))] {
        let body = request(
            p.id(),
            revision,
            &format!("sign-{revision}"),
            zero_operations(value.clone()),
        );
        let committed = p.apply_batch(body.clone()).unwrap();
        assert_eq!(committed.committed_revision, revision + 1);
        assert_eq!(committed.changed_ids, ["box", "zero"]);
        let (_, reconnected) = setup();
        let snapshot = reconnected
            .open_project(p.id())
            .unwrap()
            .snapshot()
            .unwrap();
        let number = &snapshot
            .tokens
            .iter()
            .find(|t| t.id == "zero")
            .unwrap()
            .value;
        let shadow_x =
            &snapshot.nodes.iter().find(|n| n.id == "box").unwrap().props["shadow"][0]["x"];
        let expected = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::to_string(number).unwrap(), expected);
        assert_eq!(serde_json::to_string(shadow_x).unwrap(), expected);
        let after = versions(p.id());
        for (key, old_version) in &before {
            if matches!(
                (key.0.as_str(), key.1.as_str()),
                ("token", "zero") | ("node", "box")
            ) {
                assert_ne!(old_version, &after[key], "changed row skipped: {key:?}");
            } else {
                assert_eq!(old_version, &after[key], "unchanged row rewritten: {key:?}");
            }
        }
        let history = p.commits_after(revision, 10).unwrap();
        assert_eq!(history.len(), 1);
        for recorded in [
            &history[0].operations[0]["token"]["value"],
            &history[0].operations[1]["props"]["shadow"][0]["x"],
        ] {
            assert_eq!(serde_json::to_string(recorded).unwrap(), expected);
        }
        let replay = p.apply_batch(body).unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.committed_revision, committed.committed_revision);
        assert_eq!(versions(p.id()), after);
        before = after;
    }
    p.apply_batch(request(
        p.id(),
        3,
        "identical",
        zero_operations(json!(-0.0)),
    ))
    .unwrap();
    assert_eq!(
        versions(p.id()),
        before,
        "identical JSON must not rewrite entities"
    );
    assert_eq!(p.commits_after(0, 10).unwrap().len(), 4);
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn resource_arrays_and_project_list_use_rust_order_independent_of_database_collation() {
    let (_, storage) = setup();
    let p = project(&storage, "ordering");
    let input = ["a_b", "ab", "a-b", "a", "A"];
    let mut expected: Vec<String> = input.iter().map(|id| (*id).into()).collect();
    expected.sort();
    let mut operations = page("Ordering");
    for id in input {
        operations.extend(
            serde_json::from_value::<Vec<Operation>>(json!([
                {"op":"upsert_token","token":{"id":id,"name":id,"type":"color","value":"#000000"}},
                {"op":"upsert_style","style":{"id":id,"name":id,"kind":"fill","value":"#000000"}},
                {"op":"create_note","id":id,"nodeId":"page","text":id}
            ]))
            .unwrap(),
        );
    }
    p.apply_batch(request(p.id(), 0, "seed", operations))
        .unwrap();
    let mut font = p.snapshot().unwrap().fonts.remove(0);
    // Imported asset IDs are content addressed. Insert valid serialized manifests
    // directly so this read-contract fixture can exercise arbitrary resource IDs.
    let mut client = sql();
    let database_order: Vec<String> = client
        .query(
            "SELECT id FROM unnest($1::text[]) AS fixture(id) ORDER BY id",
            &[&expected],
        )
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect();
    assert_ne!(
        database_order, expected,
        "fresh fixture must exercise nonbinary database collation"
    );
    let mut tx = client.transaction().unwrap();
    for id in input {
        let asset = Asset {
            id: id.into(),
            content_hash: "0".repeat(64),
            media_type: "image/png".into(),
            byte_size: 1,
            relative_key: "00/fixture.png".into(),
            width: Some(1),
            height: Some(1),
            status: "missing".into(),
        };
        font.id = id.into();
        for (kind, data) in [
            ("asset", serde_json::to_string(&asset).unwrap()),
            ("font", serde_json::to_string(&font).unwrap()),
        ] {
            tx.execute("INSERT INTO opencanvas_runtime_v1.entities(project_id,kind,id,data) VALUES($1,$2,$3,$4)", &[&p.id(),&kind,&id,&data]).unwrap();
        }
    }
    tx.commit().unwrap();
    let (_, reconnected) = setup();
    let snapshot = reconnected
        .open_project(p.id())
        .unwrap()
        .snapshot()
        .unwrap();
    for actual in [
        snapshot
            .tokens
            .iter()
            .map(|v| v.id.clone())
            .collect::<Vec<_>>(),
        snapshot.styles.iter().map(|v| v.id.clone()).collect(),
        snapshot.assets.iter().map(|v| v.id.clone()).collect(),
        snapshot.notes.iter().map(|v| v.id.clone()).collect(),
    ] {
        assert_eq!(actual, expected);
    }
    let mut expected_fonts = expected;
    expected_fonts.push("font_bundled".into());
    expected_fonts.sort();
    assert_eq!(
        snapshot
            .fonts
            .iter()
            .map(|v| v.id.clone())
            .collect::<Vec<_>>(),
        expected_fonts
    );

    let prefix = format!("order-{}", uuid::Uuid::new_v4().simple());
    let mut expected_slugs = vec![];
    for suffix in ["ab", "a-b", "aa", "a"] {
        let slug = format!("{prefix}-{suffix}");
        storage.create_project(&slug, "Ordering").unwrap();
        expected_slugs.push(slug);
    }
    expected_slugs.sort();
    let listed: Vec<_> = reconnected
        .list_projects()
        .unwrap()
        .into_iter()
        .map(|p| p.slug)
        .collect();
    let mut sorted = listed.clone();
    sorted.sort();
    assert_eq!(listed, sorted);
    assert_eq!(
        listed
            .into_iter()
            .filter(|s| s.starts_with(&prefix))
            .collect::<Vec<_>>(),
        expected_slugs
    );
}
