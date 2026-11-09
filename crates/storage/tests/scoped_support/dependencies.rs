use super::*;

pub fn dependencies(p: Arc<ProjectHandle>) {
    seed(&p);
    let context = edit_context(p.snapshot().unwrap(), vec!["p1".into()]).unwrap();
    assert!(context["dependencies"]
        .as_array()
        .unwrap()
        .contains(&json!("token:base")));
    assert!(context["dependencies"]
        .as_array()
        .unwrap()
        .contains(&json!("page:library")));
    strict(&p, "unrelated", vec![token("unused", json!("#ffffff"))]);
    p.apply_scoped_batch(scoped(
        &p,
        1,
        "unrelated-ok",
        &["p1"],
        vec![update("r1", json!({"name":"ok"}))],
    ))
    .unwrap();
    for (i, operation) in [
        token("base", json!("#aabbcc")),
        Operation::UpsertStyle {
            style: Style {
                id: "fill".into(),
                name: "Changed".into(),
                kind: "fill".into(),
                value: json!({"tokenId":"alias"}),
            },
        },
        update("leaf_box", json!({"name":"nested changed"})),
    ]
    .into_iter()
    .enumerate()
    {
        let base = p.snapshot().unwrap().revision;
        strict(&p, &format!("dep{i}"), vec![operation]);
        let req = scoped(
            &p,
            base,
            &format!("stale-dep{i}"),
            &["p1"],
            vec![update("r1", json!({"name":"bad"}))],
        );
        assert_eq!(
            p.apply_scoped_batch(req.clone()).unwrap_err().code,
            "REVISION_CONFLICT"
        );
        assert_eq!(
            p.dry_run_scoped_batch(req).unwrap_err().code,
            "REVISION_CONFLICT"
        );
    }
    font(&p, "font-initial", "Scoped Font", false);
    strict(&p,"font-styles",serde_json::from_value(json!([
        {"op":"upsert_style","style":{"id":"base-text","name":"Base","kind":"text","value":{"fontId":"font_bundled"}}},
        {"op":"upsert_style","style":{"id":"override-text","name":"Override","kind":"text","value":{"fontFamily":"Scoped Font"}}},
        {"op":"create_node","id":"label","kind":"text","parentId":"master","props":{"text":"Hi","width":50,"height":20,"fontSize":12,"lineHeight":16,"styleId":"base-text"}},
        {"op":"update_node","id":"instance","props":{"overrides":{"label":{"styleId":"override-text"}}}}
    ])).unwrap());
    let base = p.snapshot().unwrap().revision;
    let context = edit_context(p.snapshot().unwrap(), vec!["p1".into()]).unwrap();
    assert!(
        context["dependencies"]
            .as_array()
            .unwrap()
            .contains(&json!("font:uploaded")),
        "{context}"
    );
    font(&p, "font-reregister", "Scoped Font", true);
    assert_eq!(
        p.apply_scoped_batch(scoped(
            &p,
            base,
            "font-stale",
            &["p1"],
            vec![update("r1", json!({"name":"bad"}))]
        ))
        .unwrap_err()
        .code,
        "REVISION_CONFLICT"
    );
    // The operation's new dependency must also be protected against changes
    // since the design read, even if the old page did not reference it.
    let base = p.snapshot().unwrap().revision;
    strict(&p, "unused-change", vec![token("unused", json!("#123456"))]);
    assert_eq!(
        p.apply_scoped_batch(scoped(
            &p,
            base,
            "new-ref",
            &["p2"],
            vec![update("r2", json!({"fill":{"tokenId":"unused"}}))]
        ))
        .unwrap_err()
        .code,
        "REVISION_CONFLICT"
    );
}
