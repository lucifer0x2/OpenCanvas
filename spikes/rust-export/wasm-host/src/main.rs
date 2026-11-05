//! `wasm-host <module.wasm> <scenes dir> [--iters N] [--native DIR] [--dump DIR] [--json PATH]
//! [--font TTF] [--image PNG]` (Task 09): runs the cases of `rust-spike frame-bench` (src/frame_bench.rs)
//! inside wasmtime 50 (Cranelift, default Config). Module compile is timed once; then per case a fresh
//! instance (instantiate), `init` (font parse + PNG decode), `load_scene` (copy + JSON parse), one cold
//! `render` and N (20) warm ones. Each render sample is the exported call only (paint, pixmap stays in
//! linear memory). With --native the RGBA is compared to `<DIR>/<case>.rgba` from `frame-bench --dump`.
//! Timings are Cranelift, not V8 in a Web Worker: indicative only, not the browser gate.
mod diff;
mod module;

use module::{ms, Spike};
use serde_json::{json, Value};
use std::time::Instant;
use wasmtime::{Engine, Module, Result};

const CASES: [(&str, f32); 6] = [("spike", 1.0), ("spike", 2.0), ("frame", 1.0), ("frame", 2.0), ("tile", 1.0), ("tile", 2.0)];
const SPIKE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).map(|i| args[i + 1].as_str())
}

/// Same statistics as frame_bench.rs `summary`.
fn summary(cold: f64, mut warm: Vec<f64>) -> Value {
    warm.sort_by(f64::total_cmp);
    let rank = |q: f64| warm[((q * warm.len() as f64).ceil() as usize).max(1) - 1];
    let mean = warm.iter().sum::<f64>() / warm.len() as f64;
    json!({"coldMs": cold, "p50Ms": rank(0.5), "p95Ms": rank(0.95), "meanMs": mean, "minMs": warm[0], "warm": warm.len()})
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (path, dir) = (&args[1], &args[2]);
    let iters: usize = opt(&args, "--iters").map_or(20, |v| v.parse().unwrap());
    let font = std::fs::read(opt(&args, "--font").map_or(format!("{SPIKE}/../../renderer/fonts/NotoSans.ttf"), String::from))?;
    let png = std::fs::read(opt(&args, "--image").map_or(format!("{SPIKE}/data/asset.png"), String::from))?;
    let bytes = std::fs::read(path)?;
    let engine = Engine::default();
    let t = Instant::now();
    let module = Module::new(&engine, &bytes)?;
    let compile_ms = ms(t);
    println!("{path}: {} bytes, compile {compile_ms:.1} ms", bytes.len());
    let mut cases = Vec::new();
    for (name, scale) in CASES {
        let json = std::fs::read(format!("{dir}/{name}.json"))?;
        let t = Instant::now();
        let mut spike = Spike::new(&engine, &module)?;
        let instantiate_ms = ms(t);
        let t = Instant::now();
        spike.init(&font, &png)?;
        let init_ms = ms(t);
        let t = Instant::now();
        spike.load_scene(&json)?;
        let scene_ms = ms(t);
        let t = Instant::now();
        spike.render(scale)?;
        let cold = ms(t);
        let mut warm = Vec::with_capacity(iters);
        for _ in 0..iters {
            let t = Instant::now();
            spike.render(scale)?;
            warm.push(ms(t));
        }
        let (w, h, px) = spike.pixels()?;
        let case = format!("{name}-s{scale}");
        let mut r = summary(cold, warm);
        r["case"] = json!(case);
        r["size"] = json!([w, h]);
        r["instantiateMs"] = json!(instantiate_ms);
        r["initMs"] = json!(init_ms);
        r["loadSceneMs"] = json!(scene_ms);
        r["memoryBytes"] = json!(spike.memory_bytes());
        if let Some(out) = opt(&args, "--dump") {
            std::fs::create_dir_all(out)?;
            std::fs::write(format!("{out}/{case}.rgba"), &px)?;
        }
        if let Some(native) = opt(&args, "--native") {
            r["vsNative"] = diff::compare(&std::fs::read(format!("{native}/{case}.rgba"))?, &px, w);
        }
        println!("{r}");
        cases.push(r);
    }
    let report = json!({
        "module": path, "bytes": bytes.len(), "runtime": "wasmtime 50.0.1 (Cranelift, default Config)",
        "platform": format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        "compileMs": compile_ms, "iters": iters, "cases": cases,
    });
    if let Some(path) = opt(&args, "--json") {
        std::fs::write(path, report.to_string())?;
    }
    Ok(())
}
