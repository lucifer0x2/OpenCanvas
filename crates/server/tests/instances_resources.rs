#[allow(dead_code)]
mod compact_support;
mod instances_support;
#[allow(dead_code)]
mod support;
use compact_support::{all, fixture, ids, pages};
use instances_support::create;
use serde_json::json;
use support::Server;

#[tokio::test]
async fn instances_compact_resources_close_over_master_and_override_styles_images_and_all_themes() {
    let s = Server::start().await;
    let (project, asset) = fixture(&s).await;
    let result = s.batch(&project, 2, "components", json!([
        {"op":"upsert_style","style":{"id":"override","name":"Override","kind":"text","value":{"fontId":"font_bundled","fontSize":18,"lineHeight":24,"fill":{"tokenId":"brand"}}}},
        create("target", "frame", Some("page"), json!({"width":300,"height":300})),
        create("master", "frame", Some("page"), json!({"component":{"name":"Card"},"width":100,"height":100})),
        create("label", "text", Some("master"), json!({"text":"base","styleId":"heading"})),
        create("image", "image", Some("master"), json!({"assetId":asset,"width":20,"height":20})),
        create("instance", "instance", Some("target"), json!({"componentId":"master","overrides":{"label":{"styleId":"override","text":"changed","visible":false},"image":{"assetId":asset,"name":"override image"}}}))
    ])).await;
    assert_eq!(result["committedRevision"], 3, "{result}");
    for theme in ["default", "night"] {
        let reference = pages(
            &s,
            json!({"projectId":project,"frameId":"target","compact":true,"theme":theme}),
        )
        .await;
        // The original definition is reusable even though every placed instance overrides it.
        assert_eq!(ids(&all(&reference, "styles")), vec!["heading", "override"]);
        assert_eq!(
            ids(&all(&reference, "tokens")),
            vec!["brand", "brand-night", "ink", "size-l"]
        );
        assert_eq!(ids(&all(&reference, "fonts")), vec!["font_bundled"]);
        assert_eq!(ids(&all(&reference, "assets")), vec![asset.clone()]);
        let expanded = pages(
            &s,
            json!({"projectId":project,"frameId":"target","instances":"expand","theme":theme}),
        )
        .await;
        let nodes = all(&expanded, "nodes");
        let label = nodes.iter().find(|n| n["id"] == "instance/label").unwrap();
        assert_eq!(
            label["resolvedProps"]["fill"],
            if theme == "night" {
                "#223344"
            } else {
                "#0055ff"
            }
        );
        assert_eq!(label["resolvedProps"]["text"], "changed");
        assert_eq!(label["resolvedProps"]["opacity"], 0);
        let image = nodes.iter().find(|n| n["id"] == "instance/image").unwrap();
        assert_eq!(image["resolvedProps"]["assetId"], asset);
        assert_eq!(image["resolvedProps"]["name"], "override image");
    }
    let failed = s.batch(&project, 3, "missing-image", json!([{"op":"update_node","id":"instance","props":{"overrides":{"image":{"assetId":"missing"}}}}])).await;
    assert_eq!(failed["error"]["field"], "props.overrides.image.assetId");
    assert_eq!(
        s.get(&format!("/api/v1/projects/{project}/snapshot")).await["revision"],
        3
    );
}
