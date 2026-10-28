use crate::support::Server;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub fn viewports() -> Value {
    json!([{"width":1040,"height":720},{"width":800,"height":720}])
}

pub async fn settled(s: &Server, args: Value) -> Value {
    let first = s.call("get_design_context", args.clone()).await;
    assert!(first.get("error").is_none(), "{first}");
    for item in first["computed"]["viewports"].as_array().unwrap() {
        if let Some(id) = item["jobId"].as_str() {
            s.wait_job(id).await;
        }
    }
    s.call("get_design_context", args).await
}

pub async fn sample(s: &Server, stacks: bool) -> (String, String, usize) {
    let script = if stacks {
        "import {stacksOperations as operations,FRAME} from './samples/welcome-stacks/design.mjs'; console.log(JSON.stringify({operations:operations(),frame:FRAME}));"
    } else {
        "import {welcomeOperations as operations,FRAME} from './samples/welcome/design.mjs'; console.log(JSON.stringify({operations:operations(),frame:FRAME}));"
    };
    let output = std::process::Command::new("node")
        .args(["--input-type=module", "-e", script])
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let fixture: Value = serde_json::from_slice(&output.stdout).unwrap();
    let nodes = fixture["operations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|op| op["op"] == "create_node")
        .count()
        - 1;
    let project = s
        .call(
            "create_project",
            json!({"slug":if stacks {"stacks"} else {"free"},"name":"Temporary sample"}),
        )
        .await;
    let p = project["projectId"].as_str().unwrap().to_owned();
    assert_eq!(
        s.batch(&p, 0, "sample", fixture["operations"].clone())
            .await["committedRevision"],
        1
    );
    (p, fixture["frame"].as_str().unwrap().into(), nodes)
}

pub async fn wait_context(s: &Server, args: Value) -> (Value, Duration) {
    let start = Instant::now();
    let result = settled(s, args).await;
    (result, start.elapsed())
}
