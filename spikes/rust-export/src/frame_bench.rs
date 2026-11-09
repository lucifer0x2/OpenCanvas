//! `frame-bench <dir> [--image PNG] [--iters N] [--threads N] [--dump DIR] [--json PATH]` (Task 09):
//! native timing of the scenes `frames` wrote, the reference for the wasm host. Per case a fresh
//! Shaper + Painter (font parse not timed), one cold render, then N (20) warm renders; each sample
//! is paint only (scene JSON parsed once, outside the timer). --dump writes every case's
//! premultiplied RGBA (the tiny-skia pixmap bytes) as `<DIR>/<case>-s<scale>.rgba` for `wasm-host`.
//! Also checks the tile against the matching quarter of the frame (informational).
use crate::assets::FONT_PATH;
use crate::font::Shaper;
use crate::mode::PaintMode;
use crate::paint::Painter;
use crate::par::{self, Pool};
use crate::scene::Scene;
use crate::text::{TextPainter, EXPORT_OPTS};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Instant;
use tiny_skia::Pixmap;

/// (scene file stem, device pixel ratio); the wasm host runs the same list.
pub const CASES: [(&str, f32); 6] = [("spike", 1.0), ("spike", 2.0), ("frame", 1.0), ("frame", 2.0), ("tile", 1.0), ("tile", 2.0)];

fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).map(|i| args[i + 1].as_str())
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

/// Cold, then p50/p95 (nearest rank), mean and min of the warm samples, ms.
pub fn summary(cold: f64, mut warm: Vec<f64>) -> Value {
    assert!(!warm.is_empty(), "--iters must be positive");
    let samples = warm.clone();
    warm.sort_by(f64::total_cmp);
    let rank = |q: f64| warm[((q * warm.len() as f64).ceil() as usize).max(1) - 1];
    let mean = warm.iter().sum::<f64>() / warm.len() as f64;
    json!({"coldMs": cold, "p50Ms": rank(0.5), "p95Ms": rank(0.95), "meanMs": mean, "minMs": warm[0], "warm": warm.len(), "samplesMs": samples})
}

/// Pixels of `tile` that differ from the bottom-right quarter of `frame`, and the max channel Δ.
fn vs_crop(frame: &Pixmap, tile: &Pixmap) -> Value {
    let (fw, tw) = (frame.width() as usize, tile.width() as usize);
    let (ox, oy) = (fw - tw, frame.height() as usize - tile.height() as usize);
    let (mut n, mut max) = (0u64, 0u8);
    for (y, row) in tile.data().chunks_exact(tw * 4).enumerate() {
        let at = ((oy + y) * fw + ox) * 4;
        for (a, b) in row.as_chunks::<4>().0.iter().zip(frame.data()[at..at + tw * 4].as_chunks::<4>().0) {
            let d = a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
            n += (d > 0) as u64;
            max = max.max(d);
        }
    }
    json!({"pixelsDiffer": n, "maxDelta": max})
}

pub fn main(args: &[String]) {
    let dir = &args[2];
    let iters: usize = opt(args, "--iters").map_or(20, |v| v.parse().unwrap());
    let font = std::fs::read(FONT_PATH).expect("font");
    let image = Pixmap::load_png(opt(args, "--image").unwrap_or("data/asset.png")).expect("image");
    let pool = Pool::new(par::requested(args));
    let mut frames: HashMap<String, Pixmap> = HashMap::new();
    let mut cases = Vec::new();
    for (name, scale) in CASES {
        let text = std::fs::read_to_string(format!("{dir}/{name}.json")).expect("run `frames` first");
        let scene: Scene = serde_json::from_str(&text).expect("scene JSON");
        let (cold, warm, px, prof) = pool.install(|| {
            let shaper = Shaper::from_bytes(font.clone());
            let painter = Painter { shaper: &shaper, text: TextPainter::new(&shaper), image: image.clone(), opts: EXPORT_OPTS, mode: PaintMode::Bbox, prof: Default::default() };
            let t = Instant::now();
            let px = painter.render(&scene, scale);
            let cold = ms(t);
            let warm: Vec<f64> = (0..iters)
                .map(|_| {
                    let t = Instant::now();
                    std::hint::black_box(painter.render(&scene, scale));
                    ms(t)
                })
                .collect();
            let prof = *painter.prof.lock().unwrap();
            (cold, warm, px, prof)
        });
        let case = format!("{name}-s{scale}");
        if let Some(out) = opt(args, "--dump") {
            std::fs::create_dir_all(out).unwrap();
            std::fs::write(format!("{out}/{case}.rgba"), px.data()).unwrap();
        }
        let mut r = summary(cold, warm);
        r["case"] = json!(case);
        r["size"] = json!([px.width(), px.height()]);
        // Last warm render; the opacity layers (alloc + composite) are the part no bucket times.
        r["profile"] = json!(prof);
        if name == "tile" {
            r["vsFrameCrop"] = vs_crop(&frames[&format!("frame-s{scale}")], &px);
        }
        println!("{r}");
        frames.insert(case, px);
        cases.push(r);
    }
    let report = json!({
        "platform": format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        "parallelFeature": cfg!(feature = "parallel"), "threads": pool.threads(), "iters": iters,
        "textOpts": {"gamma": EXPORT_OPTS.gamma, "snap": EXPORT_OPTS.snap}, "cases": cases,
    });
    if let Some(path) = opt(args, "--json") {
        std::fs::write(path, report.to_string()).unwrap();
    }
}
