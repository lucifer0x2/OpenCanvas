use super::*;

#[test]
fn corrupted_snapshot_cycle_and_unknown_theme_fail_without_recursion() {
    let (_temp, _store, p) = setup();
    let snapshot = p.snapshot().unwrap();
    for error in [
        resolved_nodes_for_theme(&snapshot, "dark").unwrap_err(),
        resolved_tokens_for_theme(&snapshot, "dark").unwrap_err(),
    ] {
        assert_eq!(error.code, "UNSUPPORTED_THEME");
        assert_eq!(error.field.as_deref(), Some("theme"));
        assert!(!error.retryable);
    }
    assert!(resolved_nodes(&snapshot).is_ok());
    let mut corrupt = snapshot;
    corrupt.tokens = ["a", "b"]
        .into_iter()
        .zip(["b", "a"])
        .map(|(id, target)| Token {
            id: id.into(),
            name: id.into(),
            token_type: "color".into(),
            value: json!({"aliasId":target}),
        })
        .collect();
    assert_eq!(
        resolved_nodes(&corrupt).unwrap_err().code,
        "VALIDATION_FAILED"
    );
    assert_eq!(
        resolved_tokens_for_theme(&corrupt, "default")
            .unwrap_err()
            .code,
        "VALIDATION_FAILED"
    );
}
