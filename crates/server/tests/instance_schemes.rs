#[allow(dead_code)]
mod export_support;
#[allow(dead_code)]
mod support;
use export_support::{decode, pixel};
use serde_json::{json, Value};
use support::Server;

fn node<'a>(nodes: &'a Value, id: &str) -> &'a Value {
    nodes
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap()
}

async fn fixture(s: &Server) -> String {
    let p = s.project().await;
    let result = s.batch(&p, 0, "fixture", json!([
        {"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":{"default":"#111111","themes":{"night":"#eeeeee","day":"#008800"}}}},
        {"op":"upsert_style","style":{"id":"paint","name":"Paint","kind":"fill","value":{"tokenId":"ink"}}},
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":240,"height":80,"theme":"night","fill":"#fff"}},
        {"op":"create_node","id":"master","kind":"frame","parentId":"page","props":{"component":{"name":"Tile"},"width":60,"height":60,"fill":{"tokenId":"ink"},"stroke":{"color":{"tokenId":"ink"},"width":2}}},
        {"op":"create_node","id":"box","kind":"rectangle","parentId":"master","props":{"x":10,"y":10,"width":20,"height":20,"fill":{"styleId":"paint"},"shadow":[{"x":1,"color":{"tokenId":"ink"}}],"states":{"hover":{"fill":{"tokenId":"ink"},"opacity":1},"pressed":{"opacity":0.5},"focus":{"stroke":{"color":{"tokenId":"ink"},"width":2}}}}},
        {"op":"create_node","id":"a","kind":"instance","parentId":"frame","props":{"componentId":"master","overrides":{"box":{"states":{"hover":{"fill":"#0000ff"}}}}}},
        {"op":"create_node","id":"b","kind":"instance","parentId":"frame","props":{"componentId":"master","x":80,"theme":"day"}},
        {"op":"create_node","id":"c","kind":"instance","parentId":"frame","props":{"componentId":"master","x":160,"theme":"default","fill":"#ff0000","stroke":{"color":"#ff0000"}}}
    ])).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    p
}

async fn render(s: &Server, p: &str, state: &str) -> Value {
    let response = s.call("render_frame", json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":240,"height":80},"theme":"default","state":state})).await;
    assert!(response.get("error").is_none(), "{response}");
    let job = s.wait_job(response["jobId"].as_str().unwrap()).await;
    assert_eq!(job["status"], "ready", "{job}");
    job
}

#[tokio::test]
async fn scoped_context_resolved_states_resources_and_real_render_inspect_export_agree() {
    let s = Server::start().await;
    let p = fixture(&s).await;
    for compact in [false, true] {
        let context = s.call("get_design_context", json!({"projectId":p,"frameId":"frame","instances":"expand","statesMode":"resolved","theme":"default","compact":compact})).await;
        assert!(context.get("error").is_none(), "{context}");
        let a = node(&context["nodes"], "a/box");
        assert_eq!(a["effectiveTheme"], "night");
        assert_eq!(a["resolvedProps"]["fill"], "#eeeeee");
        assert!(a["resolvedProps"].get("effectiveTheme").is_none());
        assert_eq!(a["resolvedStates"]["hover"]["fill"], "#0000ff");
        assert_eq!(a["resolvedStates"]["focus"]["stroke"]["color"], "#eeeeee");
        assert_eq!(a["resolvedStates"]["pressed"]["opacity"], 0.5);
        assert_eq!(
            node(&context["nodes"], "b/box")["resolvedProps"]["fill"],
            "#008800"
        );
        let c = node(&context["nodes"], "c");
        assert_eq!(
            c["resolvedProps"]
                .get("fill")
                .unwrap_or(&c["props"]["fill"]),
            "#ff0000"
        );
        assert_eq!(
            c["resolvedProps"]
                .get("stroke")
                .unwrap_or(&c["props"]["stroke"])["width"],
            2
        );
        assert_eq!(context["tokens"][0]["resolvedValue"], "#111111");
    }
    let resting = render(&s, &p, "default").await;
    let hover = render(&s, &p, "hover").await;
    for (id, theme, fill) in [
        ("a", "night", "#eeeeee"),
        ("b", "day", "#008800"),
        ("c", "default", "#ff0000"),
    ] {
        let painted = node(&resting["scene"]["nodes"], id);
        assert_eq!(painted["effectiveTheme"], theme);
        assert_eq!(painted["fill"], fill);
    }
    assert_eq!(node(&hover["scene"]["nodes"], "a/box")["fill"], "#0000ff");
    assert_eq!(
        node(&resting["scene"]["nodes"], "a/box")["shadow"][0]["color"],
        "#eeeeee"
    );
    assert_eq!(render(&s, &p, "default").await["jobId"], resting["jobId"]);
    let inspect = s.call("inspect_layout", json!({"projectId":p,"jobId":resting["jobId"],"sceneKey":resting["sceneKey"],"fields":["bounds"]})).await;
    assert_eq!(node(&inspect["nodes"], "b/box")["effectiveTheme"], "day");
    let bytes = s
        .client
        .get(resting["imageUrl"].as_str().unwrap())
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let image = decode(&bytes);
    assert_eq!(pixel(&image, 40, 40), [238, 238, 238, 255]);
    assert_eq!(pixel(&image, 120, 40), [0, 136, 0, 255]);
    assert_eq!(pixel(&image, 200, 40), [255, 0, 0, 255]);
    let exported = s.call("export_nodes", json!({"projectId":p,"nodeIds":["frame"],"theme":"default","settings":[{"format":"svg"}]})).await;
    let item = &exported["exports"][0];
    assert_eq!(item["status"], "ready", "{exported}");
    let svg = s
        .client
        .get(item["files"][0]["url"].as_str().unwrap())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    for color in ["#eeeeee", "#008800", "#ff0000"] {
        assert!(svg.contains(color), "{svg}");
    }
    assert_eq!(
        s.get(&format!("/api/v1/projects/{p}/snapshot")).await["revision"],
        1
    );
}

#[tokio::test]
async fn compact_scope_keeps_the_declaration_of_an_inherited_otherwise_unused_theme() {
    let s = Server::start().await;
    let p = s.project().await;
    let made = s.batch(&p, 0, "fixture", json!([
        {"op":"upsert_token","token":{"id":"aliasTarget","name":"Alias target","type":"color","value":"#000"}},
        {"op":"upsert_token","token":{"id":"declaration","name":"Theme declaration","type":"color","value":{"default":"#000","themes":{"night":{"aliasId":"aliasTarget"}}}}},
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"outer","kind":"frame","parentId":"page","props":{"width":100,"height":100,"theme":"night"}},
        {"op":"create_node","id":"inner","kind":"frame","parentId":"outer","props":{"width":40,"height":40,"fill":"#fff"}}
    ])).await;
    assert_eq!(made["committedRevision"], 1, "{made}");
    let context = s
        .call(
            "get_design_context",
            json!({"projectId":p,"frameId":"inner","compact":true}),
        )
        .await;
    assert!(context.get("error").is_none(), "{context}");
    assert_eq!(context["nodes"][0]["effectiveTheme"], "night");
    let ids: Vec<_> = context["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["aliasTarget", "declaration"]);
}
