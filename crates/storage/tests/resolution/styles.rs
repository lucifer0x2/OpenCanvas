use super::*;

#[test]
fn themed_text_style_inherits_tokens_and_authored_overrides_win() {
    let (_temp, _store, p) = setup();
    let operations = vec![
        token(
            "size",
            "dimension",
            json!({"default":16,"themes":{"dark":24}}),
        ),
        token("leading", "dimension", json!({"aliasId":"size"})),
        token(
            "ink",
            "color",
            json!({"default":"#000","themes":{"dark":"#fff"}}),
        ),
        Operation::UpsertStyle {
            style: Style {
                id: "type".into(),
                name: "Type".into(),
                kind: "text".into(),
                value: json!({"fontId":"font_bundled","fontSize":{"tokenId":"size"},"lineHeight":{"tokenId":"leading"},"fill":{"tokenId":"ink"}}),
            },
        },
        create("page", NodeKind::Page, None, json!({})),
        create(
            "frame",
            NodeKind::Frame,
            Some("page"),
            json!({"width":300,"height":200}),
        ),
        create(
            "text",
            NodeKind::Text,
            Some("frame"),
            json!({"text":"Theme","width":100,"height":"hug","styleId":"type","fontSize":20}),
        ),
    ];
    p.apply_batch(request(&p, 0, "text-style", operations))
        .unwrap();
    let authored = p.snapshot().unwrap();
    for (theme, leading, ink) in [("default", 16, "#000"), ("dark", 24, "#fff")] {
        let resolved = resolved_nodes_for_theme(&authored, theme).unwrap();
        let text = resolved.iter().find(|n| n.id == "text").unwrap();
        assert_eq!(text.props["fontSize"], 20, "Node override wins");
        assert_eq!(text.props["lineHeight"], leading);
        assert_eq!(text.props["fill"], ink);
        assert!(text.props.get("styleId").is_none());
    }
    assert_eq!(p.snapshot().unwrap(), authored);
    assert_eq!(
        authored.styles[0].value["lineHeight"],
        json!({"tokenId":"leading"})
    );
    let invalid = token(
        "size",
        "dimension",
        json!({"default":16,"themes":{"dark":0}}),
    );
    assert!(p
        .apply_batch(request(&p, 1, "bad-style", vec![invalid]))
        .is_err());
    assert_eq!(
        p.snapshot().unwrap(),
        authored,
        "Unused invalid style branch is also atomic"
    );
}
