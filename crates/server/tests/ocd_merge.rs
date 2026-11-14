//! `@set` partial writes merge into the stored object (overrides, stroke), JSON `merge:true`
//! deletes with `null`, and mistyped instance keys fail with a suggestion: in OCD when the
//! master is in the same source, in storage for a stored master or a JSON batch.
#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use support::Server;

const SEED: &str = "page#p
  comp#input 200x40 stroke=#E5E7EB,1,inside row p=0,12
    rect#ph 10x10
    rect#title 10x10
  frame#screen 400x300 col gap=8
    use#email input ph.fill=#111111 title.visible=true
";

fn args(project: &str, revision: u64, key: &str, input: (&str, Value)) -> Value {
    let mut args = json!({"projectId":project,"expectedRevision":revision,"idempotencyKey":key});
    args[input.0] = input.1;
    args
}

async fn props(s: &Server, p: &str, id: &str) -> Value {
    let read = s
        .call("get_subtree", json!({"projectId":p,"nodeId":id}))
        .await;
    read["nodes"][0]["props"].clone()
}

async fn seeded() -> (Server, String) {
    let s = Server::start().await;
    let p = s.project().await;
    let seed = s
        .call("apply_batch", args(&p, 0, "seed", ("source", json!(SEED))))
        .await;
    assert_eq!(seed["committedRevision"], 1, "{seed}");
    (s, p)
}

#[tokio::test]
async fn set_patches_one_override_and_part_of_a_stroke() {
    let (s, p) = seeded().await;
    let source = "@set email ph.fill=#ff0000\n@set input stroke.width=2\n@set input stroke=#000000";
    let mut edit = args(&p, 1, "edit", ("source", json!(source)));
    edit["pageIds"] = json!(["p"]);
    let edited = s.call("apply_batch", edit).await;
    assert_eq!(edited["committedRevision"], 2, "{edited}");
    assert_eq!(
        props(&s, &p, "email").await["overrides"],
        json!({"ph":{"fill":"#ff0000"},"title":{"visible":true}})
    );
    assert_eq!(
        props(&s, &p, "input").await["stroke"],
        json!({"color":"#000000","width":2,"align":"inside"})
    );
    // JSON merge:true deletes with null; a whole-object update still replaces.
    let ops = json!([
        {"op":"update_node","id":"screen","merge":true,"props":{"layout":{"gap":null}}},
        {"op":"update_node","id":"email","props":{"overrides":{"title":{"visible":false}}}}
    ]);
    let result = s.batch(&p, 2, "json", ops).await;
    assert_eq!(result["committedRevision"], 3, "{result}");
    assert_eq!(
        props(&s, &p, "screen").await["layout"],
        json!({"mode":"column"})
    );
    assert_eq!(
        props(&s, &p, "email").await["overrides"],
        json!({"title":{"visible":false}})
    );
}

#[tokio::test]
async fn mistyped_instance_keys_fail_with_a_suggestion() {
    let (s, p) = seeded().await;
    // Master in the same source: the compiler knows its descendants.
    let source = format!("{SEED}    use#pass input tittle.visible=false\n");
    let failed = s
        .call(
            "apply_batch",
            args(&p, 1, "typo", ("source", json!(source))),
        )
        .await;
    let error = &failed["error"];
    assert_eq!(error["code"], "SOURCE_PARSE_FAILED", "{failed}");
    assert_eq!((&error["line"], &error["column"]), (&json!(7), &json!(20)));
    assert_eq!(error["suggestion"], "title", "{failed}");
    // Stored master: storage names the override path and suggests the descendant.
    let source = "@in screen\n  use#pass input tittle.visible=false\n";
    let failed = s
        .call(
            "apply_batch",
            args(&p, 1, "stored", ("source", json!(source))),
        )
        .await;
    let error = &failed["error"];
    assert_eq!(error["field"], "props.overrides.tittle", "{failed}");
    // Checked on the whole batch, the error names its node, which gives the line.
    let at = (&error["nodeId"], &error["line"], &error["column"]);
    assert_eq!(at, (&json!("pass"), &json!(2), &json!(3)), "{failed}");
    assert_eq!(error["suggestion"], "title", "{failed}");
    // JSON batches keep their field paths and gain the suggestion.
    for (props, field, suggestion) in [
        (
            json!({"componentId":"input","opacty":1}),
            "props.opacty",
            "opacity",
        ),
        (
            json!({"componentId":"input","overrides":{"ph":{"fil":"#000"}}}),
            "props.overrides.ph.fil",
            "fill",
        ),
    ] {
        let op = json!([{"op":"create_node","id":"pass","kind":"instance","parentId":"screen","props":props}]);
        let failed = s.batch(&p, 1, suggestion, op).await;
        let error = &failed["error"];
        assert_eq!(
            (&error["field"], &error["suggestion"]),
            (&json!(field), &json!(suggestion)),
            "{failed}"
        );
    }
}
