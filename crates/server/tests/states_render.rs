#[allow(dead_code)]
mod support;
use serde_json::{json, Value};
use support::Server;

async fn render(s: &Server, p: &str, state: &str) -> Value {
    let pending=s.call("render_frame",json!({"projectId":p,"revision":1,"frameId":"frame","viewport":{"width":320,"height":200},"state":state})).await;
    assert!(pending.get("error").is_none(), "{pending}");
    let job = s.wait_job(pending["jobId"].as_str().unwrap()).await;
    assert_eq!(job["status"], "ready", "{job}");
    assert!(job.get("error").is_none(), "{job}");
    job
}

async fn pixels(s: &Server, job: &Value) -> Vec<u8> {
    let bytes = s
        .client
        .get(job["imageUrl"].as_str().unwrap())
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    buf.truncate(info.buffer_size());
    buf
}

#[tokio::test]
async fn states_real_png_pixels_cache_diagnostics_and_no_layout_shift() {
    let s = Server::start().await;
    let p = s.project().await;
    let ops = json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":320,"height":200,"fill":"#fff","layout":{"mode":"column","padding":20,"gap":8}}},
        {"op":"create_node","id":"button","kind":"frame","parentId":"frame","props":{"width":120,"height":60,"fill":"#123456","layout":{"mode":"column","padding":4},"stroke":{"color":"#000","width":1},"states":{"hover":{"fill":"#abcdef"},"focus":{"stroke":{"color":"#f00","width":[8,4,6,2]},"radius":5},"pressed":{"opacity":0.5}}}},
        {"op":"create_node","id":"label","kind":"text","parentId":"button","props":{"width":"fill","height":"hug","text":"Tiếng Việt button","fontId":"font_bundled","fontSize":16,"lineHeight":20}},
        {"op":"create_node","id":"sibling","kind":"rectangle","parentId":"frame","props":{"width":120,"height":30,"fill":"#ddd","states":{"pressed":{"fill":"#ff00ff"}}}}
    ]);
    assert_eq!(s.batch(&p, 0, "fixture", ops).await["committedRevision"], 1);
    let default = render(&s, &p, "default").await;
    let hover = render(&s, &p, "hover").await;
    assert_ne!(default["sceneKey"], hover["sceneKey"]);
    assert_eq!(hover["scene"]["provenance"]["state"], "hover");
    assert_eq!(hover["scene"]["nodes"][1]["fill"], "#abcdef");
    assert_eq!(
        hover["scene"]["nodes"][2]["fill"], default["scene"]["nodes"][2]["fill"],
        "child does not inherit hover"
    );
    assert_eq!(render(&s, &p, "hover").await["jobId"], hover["jobId"]);
    let a = pixels(&s, &default).await;
    let b = pixels(&s, &hover).await;
    let bounds = &default["scene"]["nodes"][1]["bounds"];
    let (x, y, w, h) = (
        bounds["x"].as_f64().unwrap(),
        bounds["y"].as_f64().unwrap(),
        bounds["width"].as_f64().unwrap(),
        bounds["height"].as_f64().unwrap(),
    );
    let mut changed = 0;
    for (i, (a, b)) in a.chunks_exact(4).zip(b.chunks_exact(4)).enumerate() {
        if a != b {
            changed += 1;
            let (px, py) = ((i % 320) as f64, (i / 320) as f64);
            assert!(
                px >= x && px < x + w && py >= y && py < y + h,
                "diff outside button at {px},{py}"
            );
        }
    }
    assert!(changed > 0);
    for state in ["hover", "pressed", "focus", "disabled"] {
        let job = render(&s, &p, state).await;
        assert_ne!(job["sceneKey"], default["sceneKey"]);
        for (a, b) in default["scene"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .zip(job["scene"]["nodes"].as_array().unwrap())
        {
            assert_eq!(a["bounds"], b["bounds"], "{state}: {}", a["id"]);
            assert_eq!(a["lines"], b["lines"], "{state}: {}", a["id"]);
        }
        let inspect = s
            .call(
                "inspect_layout",
                json!({"projectId":p,"jobId":job["jobId"],"sceneKey":job["sceneKey"]}),
            )
            .await;
        let expected = s
            .call(
                "inspect_layout",
                json!({"projectId":p,"jobId":default["jobId"],"sceneKey":default["sceneKey"]}),
            )
            .await;
        assert_eq!(inspect["nodes"], expected["nodes"], "{state}");
        if state == "disabled" {
            assert_eq!(pixels(&s, &job).await, a);
            assert!(job["diagnostics"]
                .as_array()
                .unwrap()
                .contains(&json!("STATE_NOT_DECLARED")));
        }
        if state == "focus" {
            assert_eq!(
                job["scene"]["nodes"][1]["stroke"]["width"],
                json!([8, 4, 6, 2])
            );
        }
        if state == "pressed" {
            assert_eq!(job["scene"]["nodes"][1]["opacity"], 0.5);
            assert_eq!(job["scene"]["nodes"][3]["fill"], "#ff00ff");
        }
    }
}
