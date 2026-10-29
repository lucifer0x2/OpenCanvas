//! Spike CLI: measure | wrap | make-image | render | compare. See README.md.
mod compare;
mod font;
mod paint;
mod scene;
mod text;
mod wrap;

use serde_json::{json, Value};
use std::time::Instant;

fn read_json(p: &str) -> Value { serde_json::from_str(&std::fs::read_to_string(p).expect(p)).unwrap() }
fn arg_f(args: &[String], name: &str, d: f32) -> f32 {
    args.iter().position(|a| a == name).map(|i| args[i + 1].parse().unwrap()).unwrap_or(d)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let shaper = font::Shaper::load();
    match args[1].as_str() {
        "measure" => {
            let c = read_json(&args[2]);
            let mut out = serde_json::Map::new();
            for size in c["sizes"].as_array().unwrap() {
                let s = size.as_f64().unwrap();
                let ws: Vec<f64> = c["strings"].as_array().unwrap().iter().map(|t| shaper.width(t.as_str().unwrap(), s)).collect();
                out.insert(size.to_string(), json!(ws));
            }
            std::fs::write(&args[3], Value::Object(out).to_string()).unwrap();
        }
        "measure-model" => {
            let c = read_json(&args[2]);
            let variant: u8 = args[4].parse().unwrap();
            let mut out = serde_json::Map::new();
            for size in c["sizes"].as_array().unwrap() {
                let s = size.as_f64().unwrap();
                let ws: Vec<f64> = c["strings"].as_array().unwrap().iter().map(|t| shaper.width_fixed(t.as_str().unwrap(), s, variant)).collect();
                out.insert(size.to_string(), json!(ws));
            }
            std::fs::write(&args[3], Value::Object(out).to_string()).unwrap();
        }
        "wrap" => {
            let c = read_json(&args[2]);
            let mut out = vec![];
            for (pi, p) in c["paragraphs"].as_array().unwrap().iter().enumerate() {
                for sz in c["wrap"]["sizes"].as_array().unwrap() {
                    for w in c["wrap"]["widths"].as_array().unwrap() {
                        let (s, lh) = (sz[0].as_f64().unwrap(), sz[1].as_f64().unwrap());
                        let model: u8 = args.get(4).map(|v| v.parse().unwrap()).unwrap_or(0);
                        let px = |t: &str| if model == 0 { shaper.width(t, s) } else { shaper.width_fixed(t, s, model) };
                        let lines = wrap::measure(&px, p.as_str().unwrap(), s, lh, w.as_f64());
                        out.push(json!({"p": pi, "size": s, "lineHeight": lh, "width": w, "lines": lines}));
                    }
                }
            }
            std::fs::write(&args[3], json!(out).to_string()).unwrap();
        }
        "make-image" => {
            // 32×24 gradient with a checker overlay: smooth ramps plus hard edges for filtering.
            let (w, h) = (32u32, 24u32);
            let mut px = vec![0u8; (w * h * 4) as usize];
            for y in 0..h { for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                let chk = ((x / 4 + y / 4) % 2) as u8;
                px[i] = (x * 255 / (w - 1)) as u8;
                px[i + 1] = (y * 255 / (h - 1)) as u8;
                px[i + 2] = if chk == 1 { 220 } else { 60 };
                px[i + 3] = 255;
            }}
            compare::write_png(&args[2], w, h, &px);
        }
        "render" => {
            let sc: scene::Scene = serde_json::from_str(&std::fs::read_to_string(&args[2]).unwrap()).unwrap();
            let image = tiny_skia::Pixmap::load_png(&args[3]).unwrap();
            let opts = text::TextOpts { gamma: arg_f(&args, "--gamma", 1.0), snap: args.iter().any(|a| a == "--snap") };
            let painter = paint::Painter { shaper: &shaper, text: text::TextPainter::new(&shaper), image, opts };
            let bench = arg_f(&args, "--bench", 0.0) as usize;
            for scale in [1.0f32, 2.0] {
                let px = painter.render(&sc, scale);
                px.save_png(format!("{}-s{}.png", args[4], scale)).unwrap();
                if bench > 0 {
                    let t = Instant::now();
                    for _ in 0..bench { let p = painter.render(&sc, scale); std::hint::black_box(p.encode_png().unwrap()); }
                    let e1 = t.elapsed().as_secs_f64();
                    let t2 = Instant::now();
                    for _ in 0..bench { std::hint::black_box(painter.render(&sc, scale)); }
                    println!("{}", json!({"scale": scale, "iters": bench,
                        "msRenderPlusPng": e1 * 1e3 / bench as f64,
                        "msRenderOnly": t2.elapsed().as_secs_f64() * 1e3 / bench as f64}));
                }
            }
        }
        "compare" => {
            let a = compare::read_png(&args[2]);
            let b = compare::read_png(&args[3]);
            let sc = read_json(&args[4]);
            let regions: Vec<compare::Region> = serde_json::from_value(sc["regions"].clone()).unwrap();
            let scale: f32 = args[5].parse().unwrap();
            let r = compare::compare(&a, &b, &regions, scale, args.get(6).map(|s| s.as_str()));
            println!("{r}");
        }
        other => panic!("unknown command {other}"),
    }
}
