//! The same restrictions and complete errors apply with and without stale history.
use super::*;

#[test]
fn current_revision_shortcut_keeps_stale_path_restriction_and_coverage_errors() {
    let initial = seed();
    let operations: Vec<Operation> = serde_json::from_value(json!([
        {"op":"upsert_token","token":{"id":"unused","name":"Unused","type":"number","value":1}}
    ]))
    .unwrap();
    let (candidate, changed, _) = crate::batch::validated_candidate(&initial, &operations).unwrap();
    let scope = BatchScope::new(&initial, &candidate, &changed).unwrap();
    let body = json!({"projectId":initial.project_id,"expectedRevision":1,"idempotencyKey":"unrelated","operations":operations});
    let history =
        history::envelope_with_scopes(scope.footprint, body.clone(), body["operations"].clone())
            .unwrap();
    let mut before = snapshot(&initial, &candidate);
    before.revision = 2;
    let cases = [
        json!([{"op":"update_node","id":"r1","props":{"name":"Allowed"}}]),
        json!([{"op":"reparent_node","id":"r1","parentId":"f2"}]),
        json!([{"op":"delete_subtree","id":"f2"}]),
        json!([{"op":"delete_subtree","id":"r2"}]),
        json!([{"op":"reorder_node","id":"f1","index":0}]),
        json!([{"op":"update_node","id":"f1","props":{"flowStart":"global"}}]),
        json!([{"op":"update_node","id":"p1","props":{"name":"Page"}}]),
        json!([{"op":"upsert_token","token":{"id":"base","name":"Base","type":"color","value":"#ffffff"}}]),
    ];
    for case in cases {
        for pages in [
            vec![],
            vec!["missing".into()],
            vec!["p1".into()],
            vec!["p1".into(), "p2".into()],
        ] {
            let request = BatchRequest {
                project_id: before.project_id.clone(),
                expected_revision: 2,
                idempotency_key: "guard".into(),
                operations: serde_json::from_value(case.clone()).unwrap(),
            };
            let run = |request: &BatchRequest, commits| {
                let (candidate, changed, mut work) =
                    crate::batch::validated_candidate(&before, &request.operations).unwrap();
                let scope = BatchScope::new(&before, &candidate, &changed).unwrap();
                super::super::check(
                    &before, &candidate, request, &pages, commits, &scope, &mut work,
                )
                .map_err(|e| serde_json::to_value(e).unwrap())
            };
            let current = run(&request, vec![]);
            let mut stale = request;
            stale.expected_revision = 1;
            assert_eq!(
                current,
                run(&stale, vec![(2, history.clone())]),
                "case={case}, pages={pages:?}"
            );
        }
    }
}
