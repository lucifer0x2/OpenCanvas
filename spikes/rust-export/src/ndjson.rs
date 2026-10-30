//! `ndjson <requests.ndjson> <responses.ndjson> [iters]` (Part D): what the server does per job
//! with the worker's NDJSON (crates/server/src/worker.rs, jobs/runner.rs, jobs/png.rs), replayed
//! on lines captured by a tee wrapper: serialize the compute request, parse the compute response,
//! serialize the render request around its scene, parse the render response, base64-decode the
//! PNG and check its header (runner and record.rs each decode once, so twice per job).
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::time::Instant;

fn lines(path: &str) -> Vec<String> {
    std::fs::read_to_string(path).expect(path).lines().filter(|l| !l.is_empty()).map(String::from).collect()
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn decode(value: &Value) -> usize {
    let bytes = STANDARD.decode(value["base64"].as_str().unwrap()).unwrap();
    assert!(bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" && &bytes[12..16] == b"IHDR");
    bytes.len()
}

pub fn main(args: &[String]) {
    let requests = lines(&args[2]);
    let responses = lines(&args[3]);
    let iters: usize = args.get(4).map(|v| v.parse().unwrap()).unwrap_or(20);
    let compute_lines: Vec<&String> = requests.iter().filter(|l| l.contains("\"op\":\"compute\"")).collect();
    let compute: Vec<Value> = compute_lines.iter().map(|l| serde_json::from_str::<Value>(l).unwrap()).collect();
    let mut jobs = Vec::new();
    // Responses come in request order: compute, render, compute, render, ...
    for (i, pair) in responses.chunks(2).enumerate().take(compute.len()) {
        let [computed_line, rendered_line] = pair else { break };
        let input = &compute[i]["input"];
        let mut t = [vec![], vec![], vec![], vec![], vec![]];
        let mut png_bytes = 0;
        for _ in 0..iters {
            let s = Instant::now();
            let req = serde_json::to_vec(&json!({"op":"compute","input":input,"strict":false})).unwrap();
            t[0].push(s.elapsed().as_secs_f64() * 1e3);
            let s = Instant::now();
            let computed: Value = serde_json::from_str(computed_line).unwrap();
            t[1].push(s.elapsed().as_secs_f64() * 1e3);
            let s = Instant::now();
            let render = serde_json::to_vec(&json!({"op":"render","scene":computed["scene"],"scale":1,"strict":true})).unwrap();
            t[2].push(s.elapsed().as_secs_f64() * 1e3);
            let s = Instant::now();
            let rendered: Value = serde_json::from_str(rendered_line).unwrap();
            t[3].push(s.elapsed().as_secs_f64() * 1e3);
            let s = Instant::now();
            png_bytes = decode(&rendered);
            std::hint::black_box(decode(&rendered));
            t[4].push(s.elapsed().as_secs_f64() * 1e3);
            std::hint::black_box((req, render));
        }
        jobs.push(json!({
            "computeRequestBytes": compute_lines[i].len(),
            "computeResponseBytes": computed_line.len(), "renderResponseBytes": rendered_line.len(), "pngBytes": png_bytes,
            "msSerializeCompute": median(t[0].clone()), "msParseComputed": median(t[1].clone()),
            "msSerializeRender": median(t[2].clone()), "msParseRendered": median(t[3].clone()),
            "msDecodePngTwice": median(t[4].clone()),
            "msTotal": t.iter().map(|v| median(v.clone())).sum::<f64>(),
        }));
    }
    println!("{}", json!(jobs));
}
