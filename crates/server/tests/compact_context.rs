mod compact_support;
mod support;
use compact_support::{all, bytes, fixture, ids, pages, FILLER_TOKENS};
use opencanvas_storage::{resolved_nodes_for_theme, Snapshot};
use serde_json::{json, Value};
use support::Server;

fn strs(list: &[&str]) -> Vec<String> {
    let mut list: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
    list.sort();
    list
}

#[tokio::test]
async fn compact_false_is_byte_identical_to_the_default_on_every_page() {
    let s = Server::start().await;
    let (p, _) = fixture(&s).await;
    for frame in ["card", "other"] {
        let base = json!({"projectId":p,"frameId":frame,"budget":2048,"theme":"night"});
        let mut off = base.clone();
        off["compact"] = json!(false);
        let default = pages(&s, base).await;
        assert!(default.len() > 1, "fixture must page");
        assert_eq!(default, pages(&s, off).await);
        assert!(default[0].get("resolvedPropsMode").is_none());
        assert!(default[0].get("resources").is_none());
    }
}

#[tokio::test]
async fn compact_keeps_only_differing_props_and_referenced_resources() {
    let s = Server::start().await;
    let (p, asset) = fixture(&s).await;
    for theme in ["default", "night"] {
        let args = json!({"projectId":p,"frameId":"card","theme":theme});
        let full = pages(&s, args.clone()).await;
        let mut compact_args = args.clone();
        compact_args["compact"] = json!(true);
        let compact = pages(&s, compact_args).await;
        assert_eq!(compact.len(), 1);
        let page = &compact[0];
        assert_eq!(page["resolvedPropsMode"], "differing");
        assert_eq!(page["resources"], "referenced");
        assert_eq!(page["diagnostics"], full[0]["diagnostics"]);
        let tokens = [
            "ink",
            "size-l",
            "brand",
            "brand-night",
            "radius-alias",
            "radius-base",
            "space",
        ];
        assert_eq!(ids(&all(&compact, "tokens")), strs(&tokens));
        assert_eq!(
            ids(&all(&compact, "styles")),
            strs(&["heading", "brand-fill"])
        );
        assert_eq!(ids(&all(&compact, "fonts")), strs(&["font_bundled"]));
        assert_eq!(ids(&all(&compact, "assets")), vec![asset.clone()]);
        // Kept resources are the default read's items, unchanged (resolvedValue included).
        for field in ["tokens", "styles", "fonts", "assets"] {
            let every = all(&full, field);
            for item in all(&compact, field) {
                assert!(every.contains(&item), "{field} {item}");
            }
        }
        let full_nodes = all(&full, "nodes");
        let compact_nodes = all(&compact, "nodes");
        assert_eq!(full_nodes.len(), compact_nodes.len());
        for (f, c) in full_nodes.iter().zip(&compact_nodes) {
            assert_eq!(f["props"], c["props"]);
            // Rebuild the full resolvedProps: authored without styleId, overlaid by compact.
            let mut rebuilt = c["props"].as_object().unwrap().clone();
            rebuilt.remove("styleId");
            for (k, v) in c["resolvedProps"].as_object().unwrap() {
                assert_ne!(c["props"].get(k), Some(v), "{k} equals authored");
                rebuilt.insert(k.clone(), v.clone());
            }
            assert_eq!(Value::Object(rebuilt), f["resolvedProps"], "{}", c["id"]);
        }
        let by_id = |id: &str| compact_nodes.iter().find(|n| n["id"] == id).unwrap();
        assert_eq!(by_id("photo")["resolvedProps"], json!({"fit":"fill"}));
        assert!(by_id("title")["resolvedProps"]["fontId"] == "font_bundled");
        let fill = if theme == "night" {
            "#223344"
        } else {
            "#0055ff"
        };
        assert_eq!(by_id("swatch")["resolvedProps"], json!({ "fill": fill }));
    }
}

#[tokio::test]
async fn compact_pages_carry_every_node_once_and_the_same_resource_set() {
    let s = Server::start().await;
    let (p, _) = fixture(&s).await;
    let args = json!({"projectId":p,"frameId":"card","compact":true});
    let single = pages(&s, args.clone()).await;
    let mut small = args.clone();
    small["budget"] = json!(1024);
    small["limit"] = json!(2);
    let paged = pages(&s, small).await;
    assert!(paged.len() > 2, "{}", paged.len());
    assert_eq!(all(&paged, "nodes"), all(&single, "nodes"));
    for field in ["tokens", "styles", "fonts", "assets"] {
        assert_eq!(all(&paged, field), all(&single, field), "{field}");
    }
    for page in &paged {
        assert_eq!(page["resolvedPropsMode"], "differing");
        assert_eq!(page["diagnostics"], single[0]["diagnostics"]);
    }
    let first = paged[0]["nextCursor"].clone();
    let other = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"card","budget":1024,"limit":2,"cursor":first}),
        )
        .await;
    assert_eq!(other["error"]["code"], "INVALID_CURSOR");
    let outline = s
        .call("get_outline", json!({"projectId":p,"compact":true}))
        .await;
    assert_eq!(outline["error"]["code"], "INVALID_REQUEST", "{outline}");
}

/// Pins the mirrored reference key set against storage's own resolver: the compact resources
/// alone resolve the scope to the same values for every theme, and dropping any one of them
/// makes storage validation fail.
#[tokio::test]
async fn compact_resource_set_is_exactly_what_storage_needs() {
    let s = Server::start().await;
    let (p, _) = fixture(&s).await;
    let full = s.state.snapshot(p.clone()).await.unwrap();
    for frame in ["card", "other"] {
        let args = json!({"projectId":p,"frameId":frame,"compact":true});
        let compact = pages(&s, args).await;
        let keep = |field: &str| ids(&all(&compact, field));
        let scope = ids(&all(&compact, "nodes"));
        let mut reduced: Snapshot = full.clone();
        reduced
            .nodes
            .retain(|n| n.id == "page" || scope.contains(&n.id));
        reduced.tokens.retain(|x| keep("tokens").contains(&x.id));
        reduced.styles.retain(|x| keep("styles").contains(&x.id));
        reduced.fonts.retain(|x| keep("fonts").contains(&x.id));
        reduced.assets.retain(|x| keep("assets").contains(&x.id));
        let themes: &[&str] = if keep("tokens").contains(&"brand".to_owned()) {
            &["default", "night"]
        } else {
            &["default"]
        };
        for theme in themes {
            let expected = resolved_nodes_for_theme(&full, theme).unwrap();
            let got = resolved_nodes_for_theme(&reduced, theme).unwrap();
            for node in &got {
                let want = expected.iter().find(|n| n.id == node.id).unwrap();
                assert_eq!(node.props, want.props, "{frame} {theme} {}", node.id);
            }
        }
        let mut removable = 0;
        for field in ["tokens", "styles", "fonts", "assets"] {
            for id in keep(field) {
                let mut less = reduced.clone();
                match field {
                    "tokens" => less.tokens.retain(|x| x.id != id),
                    "styles" => less.styles.retain(|x| x.id != id),
                    "fonts" => less.fonts.retain(|x| x.id != id),
                    _ => less.assets.retain(|x| x.id != id),
                }
                assert!(
                    resolved_nodes_for_theme(&less, "default").is_err(),
                    "{frame}: {field} {id} is not needed"
                );
                removable += 1;
            }
        }
        assert!(removable >= 2, "{frame}");
    }
}

/// Measures the resources part on the 300-token fixture; run with --nocapture for numbers.
#[tokio::test]
async fn compact_shrinks_a_300_token_project_context() {
    let s = Server::start().await;
    let (p, _) = fixture(&s).await;
    let args = json!({"projectId":p,"frameId":"card"});
    let full = pages(&s, args.clone()).await;
    let mut compact_args = args;
    compact_args["compact"] = json!(true);
    let compact = pages(&s, compact_args).await;
    let tokens = all(&full, "tokens").len();
    assert_eq!(tokens, FILLER_TOKENS + 8);
    assert_eq!(all(&compact, "tokens").len(), 7);
    let resource_bytes = |pages: &[Value]| -> usize {
        ["tokens", "styles", "fonts", "assets"]
            .iter()
            .flat_map(|f| all(pages, f))
            .map(|i| serde_json::to_vec(&i).unwrap().len())
            .sum()
    };
    let (fb, cb) = (bytes(&full), bytes(&compact));
    let (fr, cr) = (resource_bytes(&full), resource_bytes(&compact));
    eprintln!(
        "300-token fixture, frame card: default {} pages / {fb} B (resources {fr} B, {tokens} tokens); compact {} pages / {cb} B (resources {cr} B, 7 tokens)",
        full.len(),
        compact.len()
    );
    assert!(full.len() > compact.len());
    assert!(cb * 4 < fb, "{cb} vs {fb}");
}
