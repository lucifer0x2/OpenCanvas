#![allow(dead_code)]
use crate::support::Server;
use serde_json::{json, Value};

pub fn create(id: &str, kind: &str, parent: Option<&str>, props: Value) -> Value {
    json!({"op":"create_node","id":id,"kind":kind,"parentId":parent,"props":props})
}
pub fn row() -> Value {
    json!({"width":"fill","height":44,"layout":{"mode":"row","gap":12,"padding":[8,12]},"fill":"#f7f9fc","stroke":{"color":"#dce1ea","width":1}})
}
pub fn text(value: &str) -> Value {
    json!({"text":value,"fontId":"font_bundled","fontSize":14,"lineHeight":20,"width":150,"height":20,"fill":"#223344"})
}
pub fn operations() -> (Vec<Value>, Vec<Value>) {
    let mut master = row();
    master["component"] = json!({"name":"TableRow"});
    let mut instances = vec![create("cmpRow", "frame", Some("page"), master)];
    for (id, value) in [
        ("id", "0"),
        ("q", "select * from t0"),
        ("ms", "12ms"),
        ("st", "ok"),
    ] {
        instances.push(create(
            &format!("cmpRow-{id}"),
            "text",
            Some("cmpRow"),
            text(value),
        ));
    }
    let mut manual = Vec::new();
    for i in 0..20 {
        let id = format!("row{i}");
        instances.push(create(&id, "instance", Some("frame"), json!({"componentId":"cmpRow","width":"fill","overrides":{"cmpRow-id":{"text":i.to_string()},"cmpRow-q":{"text":format!("select * from t{i}")}}})));
        let id = format!("manual{i}");
        manual.push(create(&id, "frame", Some("manual"), row()));
        for (suffix, value) in [
            ("id", i.to_string()),
            ("q", format!("select * from t{i}")),
            ("ms", "12ms".into()),
            ("st", "ok".into()),
        ] {
            manual.push(create(
                &format!("{id}-{suffix}"),
                "text",
                Some(&id),
                text(&value),
            ));
        }
    }
    (instances, manual)
}
pub async fn fixture(s: &Server) -> String {
    let project = s.project().await;
    let mut ops = vec![create("page", "page", None, json!({}))];
    for id in ["frame", "manual"] {
        ops.push(create(
            id,
            "frame",
            Some("page"),
            json!({"width":800,"height":1000,"fill":"#ffffff","layout":{"mode":"column"}}),
        ));
    }
    let (instances, manual) = operations();
    ops.extend(instances);
    ops.extend(manual);
    let result = s.batch(&project, 0, "fixture", json!(ops)).await;
    assert_eq!(result["committedRevision"], 1, "{result}");
    project
}
pub async fn render(s: &Server, project: &str, frame: &str, revision: u64) -> Value {
    let pending = s.call("render_frame", json!({"projectId":project,"revision":revision,"frameId":frame,"viewport":{"width":800,"height":1000}})).await;
    assert!(pending["jobId"].is_string(), "{pending}");
    let ready = s.wait_job(pending["jobId"].as_str().unwrap()).await;
    assert_eq!(ready["status"], "ready", "{ready}");
    assert!(ready.get("error").is_none(), "{ready}");
    ready
}
pub async fn context(s: &Server, args: Value) -> Value {
    let first = s.call("get_design_context", args.clone()).await;
    assert!(first.get("error").is_none(), "{first}");
    if let Some(vps) = first["computed"]["viewports"].as_array() {
        for vp in vps {
            s.wait_job(vp["jobId"].as_str().unwrap()).await;
        }
        s.call("get_design_context", args).await
    } else {
        first
    }
}
pub fn bounds<'a>(job: &'a Value, id: &str) -> &'a Value {
    &job["scene"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap()["bounds"]
}
pub async fn png(s: &Server, job: &Value) -> Vec<u8> {
    s.client
        .get(job["imageUrl"].as_str().unwrap())
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap()
        .to_vec()
}
