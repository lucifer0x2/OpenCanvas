use super::*;

#[test]
fn alias_depth_and_theme_count_are_bounded() {
    let (_temp, _store, p) = setup();
    let mut operations = vec![token("base", "color", json!("#fff"))];
    for index in 1..=32 {
        let previous = if index == 1 {
            "base".into()
        } else {
            format!("alias-{}", index - 1)
        };
        operations.push(token(
            &format!("alias-{index}"),
            "color",
            json!({"aliasId":previous}),
        ));
    }
    p.apply_batch(request(&p, 0, "depth", operations)).unwrap();
    let before = p.snapshot().unwrap();
    assert_eq!(
        resolved_tokens_for_theme(&before, "default").unwrap()["alias-32"],
        "#fff"
    );
    assert!(p
        .apply_batch(request(
            &p,
            1,
            "too-deep",
            vec![token("alias-33", "color", json!({"aliasId":"alias-32"}))]
        ))
        .is_err());
    let themes: serde_json::Map<String, Value> = (0..33)
        .map(|i| (format!("theme-{i}"), json!("#fff")))
        .collect();
    assert!(p
        .apply_batch(request(
            &p,
            1,
            "too-many",
            vec![token(
                "base",
                "color",
                json!({"default":"#fff","themes":themes})
            )]
        ))
        .is_err());
    assert_eq!(p.snapshot().unwrap(), before);
    assert!(p.commits_after(1, 10).unwrap().is_empty());
}

#[test]
fn project_theme_union_accepts_32_then_rejects_33_atomically() {
    let (_temp, _store, p) = setup();
    let operations = (0..32)
        .map(|i| {
            token(
                &format!("t-{i}"),
                "color",
                json!({"default":"#fff","themes":{format!("theme-{i}"):"#123"}}),
            )
        })
        .collect();
    p.apply_batch(request(&p, 0, "themes", operations)).unwrap();
    let before = p.snapshot().unwrap();
    let resolved = resolved_tokens_for_theme(&before, "theme-31").unwrap();
    assert_eq!(resolved["t-31"], "#123");
    assert_eq!(resolved["t-0"], "#fff", "Other tokens fall back to default");
    let error = p
        .apply_batch(request(
            &p,
            1,
            "33rd",
            vec![token(
                "t-32",
                "color",
                json!({"default":"#fff","themes":{"theme-32":"#123"}}),
            )],
        ))
        .unwrap_err();
    assert_eq!(error.field.as_deref(), Some("token.themes"));
    assert_eq!(p.snapshot().unwrap(), before);
    assert!(p.commits_after(1, 10).unwrap().is_empty());
}

#[test]
fn token_count_cap_guards_untrusted_snapshot_readback() {
    let (_temp, _store, p) = setup();
    let mut snapshot = p.snapshot().unwrap();
    snapshot.tokens = (0..4096)
        .map(|i| Token {
            id: format!("t-{i}"),
            name: "Literal".into(),
            token_type: "number".into(),
            value: json!(1),
        })
        .collect();
    assert_eq!(
        resolved_tokens_for_theme(&snapshot, "default")
            .unwrap()
            .len(),
        4096
    );
    snapshot.tokens.push(Token {
        id: "excess".into(),
        name: "Excess".into(),
        token_type: "number".into(),
        value: json!(1),
    });
    let error = resolved_tokens_for_theme(&snapshot, "default").unwrap_err();
    assert_eq!(error.code, "VALIDATION_FAILED");
    assert_eq!(error.field.as_deref(), Some("tokens"));
}

#[test]
fn malformed_unused_branches_and_names_do_not_commit() {
    let (_temp, _store, p) = setup();
    for (index, value) in [
        json!({"default":"#fff"}),
        json!({"default":"#fff","themes":[]}),
        json!({"default":"#fff","themes":{},"extra":true}),
        json!({"default":"#fff","themes":{"dark":{"aliasId":false}}}),
        json!({"default":"#fff","themes":{"dark":17}}),
        json!({"default":"#fff","themes":{"é":"#000"}}),
        json!({"default":"#fff","themes":{"a".repeat(65):"#000"}}),
        json!({"aliasId":"a/b"}),
        json!(null),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(p
            .apply_batch(request(
                &p,
                0,
                &format!("bad-{index}"),
                vec![token("unused", "color", value)]
            ))
            .is_err());
        assert_eq!(p.snapshot().unwrap().revision, 0);
    }
    let mut op = token("unused", "color", json!("#fff"));
    if let Operation::UpsertToken { token } = &mut op {
        token.name = "é".repeat(129);
    }
    assert!(p
        .apply_batch(request(&p, 0, "long-name", vec![op]))
        .is_err());
    assert!(p.snapshot().unwrap().tokens.is_empty());
    assert!(p.commits_after(0, 10).unwrap().is_empty());
}
