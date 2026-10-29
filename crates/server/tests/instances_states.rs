#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use support::Server;

async fn fixture(s: &Server) -> String {
    let p = s.project().await;
    let result = s.batch(&p, 0, "fixture", json!([
        {"op":"upsert_token","token":{"id":"rootHover","name":"Root","type":"color","value":{"default":"#123456","themes":{"night":"#abcdef"}}}},
        {"op":"upsert_token","token":{"id":"childHover","name":"Child","type":"color","value":{"default":"#654321","themes":{"night":"#fedcba"}}}},
        {"op":"upsert_token","token":{"id":"unused","name":"Unused","type":"color","value":"#000000"}},
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":320,"height":200,"fill":"#ffffff"}},
        {"op":"create_node","id":"master","kind":"frame","parentId":"page","props":{"component":{"name":"Button"},"width":140,"height":60,"fill":"#eeeeee","states":{"hover":{"fill":{"tokenId":"rootHover"}}}}},
        {"op":"create_node","id":"box","kind":"rectangle","parentId":"master","props":{"x":10,"y":10,"width":20,"height":20,"fill":"#aaaaaa","states":{"hover":{"fill":{"tokenId":"childHover"}}}}},
        {"op":"create_node","id":"hidden","kind":"rectangle","parentId":"master","props":{"x":40,"y":10,"width":20,"height":20,"fill":"#bbbbbb","states":{"hover":{"fill":{"tokenId":"childHover"},"opacity":1}}}},
        {"op":"create_node","id":"label","kind":"text","parentId":"master","props":{"x":70,"y":10,"text":"Hi","fontId":"font_bundled","fontSize":14,"lineHeight":20,"states":{"hover":{"fill":"#182230"}}}},
        {"op":"create_node","id":"instance","kind":"instance","parentId":"frame","props":{"componentId":"master","x":20,"y":20,"overrides":{"hidden":{"visible":false}}}}
    ])).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    p
}

fn node<'a>(nodes: &'a Value, id: &str) -> &'a Value {
    nodes
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap()
}

#[tokio::test]
async fn instances_states_context_keeps_master_virtual_variants_and_theme_resources() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    for (theme, root, child) in [
        ("default", "#123456", "#654321"),
        ("night", "#abcdef", "#fedcba"),
    ] {
        for compact in [false, true] {
            for mode in ["reference", "expand"] {
                let context = s.call("get_design_context", json!({"projectId":p,"frameId":"frame","theme":theme,"compact":compact,"instances":mode,"statesMode":"resolved"})).await;
                assert!(context.get("error").is_none(), "{context}");
                assert_eq!(
                    node(&context["nodes"], "instance")["resolvedStates"]["hover"]["fill"],
                    root
                );
                let (nodes, prefix) = if mode == "reference" {
                    let definitions = &context["components"]["master"]["nodes"];
                    assert_eq!(
                        node(definitions, "master")["resolvedStates"]["hover"]["fill"],
                        root
                    );
                    (definitions, "")
                } else {
                    assert!(context.get("components").is_none());
                    (&context["nodes"], "instance/")
                };
                let box_node = node(nodes, &format!("{prefix}box"));
                assert_eq!(box_node["resolvedStates"]["hover"]["fill"], child);
                assert_eq!(
                    box_node["props"]["states"]["hover"]["fill"],
                    json!({"tokenId":"childHover"})
                );
                assert_eq!(
                    node(nodes, &format!("{prefix}label"))["resolvedStates"]["hover"],
                    json!({})
                );
                if compact {
                    let ids: Vec<_> = context["tokens"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|t| t["id"].as_str().unwrap())
                        .collect();
                    assert_eq!(ids, ["childHover", "rootHover"]);
                }
            }
        }
    }
    for args in [
        json!({}),
        json!({"statesMode":"resolved","properties":["fill"]}),
    ] {
        let mut request = json!({"projectId":p,"frameId":"frame"});
        request
            .as_object_mut()
            .unwrap()
            .extend(args.as_object().unwrap().clone());
        let context = s.call("get_design_context", request).await;
        assert!(node(&context["nodes"], "instance")
            .get("resolvedStates")
            .is_none());
        for definition in context["components"]["master"]["nodes"].as_array().unwrap() {
            assert!(definition.get("resolvedStates").is_none());
        }
    }
    // Inherited master variants do not broaden the minimal instance override contract.
    for (props, field) in [
        (json!({"states":{"hover":{"opacity":0.5}}}), "props.states"),
        (
            json!({"overrides":{"box":{"states":{"hover":{"opacity":0.5}}}}}),
            "props.overrides.box.states",
        ),
    ] {
        let rejected = s
            .batch(
                &p,
                1,
                "bad-state-override",
                json!([{"op":"update_node","id":"instance","props":props}]),
            )
            .await;
        assert_eq!(rejected["error"]["field"], field, "{rejected}");
        assert!(rejected["error"]["allowedKeys"].is_array());
    }
    assert_eq!(
        s.get(&format!("/api/v1/projects/{p}/snapshot")).await["revision"],
        1
    );
}

async fn render(s: &Server, p: &str, state: &str) -> Value {
    let pending = s.call("render_frame", json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":320,"height":200},"theme":"night","state":state})).await;
    assert!(pending.get("error").is_none(), "{pending}");
    let ready = s.wait_job(pending["jobId"].as_str().unwrap()).await;
    assert_eq!(ready["status"], "ready", "{ready}");
    ready
}

#[tokio::test]
async fn instances_states_render_keeps_root_child_paint_hidden_override_and_geometry() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    let default = render(&s, &p, "default").await;
    let hover = render(&s, &p, "hover").await;
    assert_ne!(default["sceneKey"], hover["sceneKey"]);
    let painted = &hover["scene"]["nodes"];
    assert_eq!(node(painted, "instance")["fill"], "#abcdef");
    assert_eq!(node(painted, "instance/box")["fill"], "#fedcba");
    assert_eq!(node(painted, "instance/hidden")["fill"], "#fedcba");
    assert_eq!(node(painted, "instance/hidden")["opacity"], 0);
    assert!(!hover["diagnostics"]
        .as_array()
        .unwrap()
        .contains(&json!("STATE_NOT_DECLARED")));
    for resting in default["scene"]["nodes"].as_array().unwrap() {
        let variant = node(painted, resting["id"].as_str().unwrap());
        assert_eq!(resting["bounds"], variant["bounds"]);
        assert_eq!(resting["lines"], variant["lines"]);
    }
    assert_eq!(render(&s, &p, "hover").await["jobId"], hover["jobId"]);
    let mut images = Vec::new();
    for job in [&default, &hover] {
        images.push(
            s.client
                .get(job["imageUrl"].as_str().unwrap())
                .send()
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap(),
        );
    }
    assert_ne!(images[0], images[1]);
}
