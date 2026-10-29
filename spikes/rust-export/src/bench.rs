//! `bench <input.json> [options]` (Part D): process startup (input, font read + SHA-256 + parse,
//! thread pool), then a viewport-width sequence in process like the server's warm jobs, each run
//! timed by stage. Options: --scale S, --iters N (25), --step PX (1), --nocache (fresh measure
//! cache per run), --gamma G (1.45), --no-snap, --paint full|bbox (bbox), --threads N (0 = one per
//! core; or RUST_SPIKE_THREADS), --png PATH (first run; PATH-fast.png too), --geometry PATH,
//! --json PATH (else stdout). With --iters 1 it is the cold measurement.
use crate::font::{Shaper, FONT_PATH};
use crate::measure::Face;
use crate::mode::PaintMode;
use crate::paint::Painter;
use crate::par::{self, Pool};
use crate::pipeline;
use crate::text::{TextOpts, TextPainter};
use opencanvas_layout::MeasureCache;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::time::Instant;

fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).map(|i| args[i + 1].as_str())
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

pub fn main(args: &[String]) {
    let start = Instant::now();
    let num = |name: &str, d: f64| opt(args, name).map(|v| v.parse::<f64>().unwrap()).unwrap_or(d);
    let (scale, iters, step) = (num("--scale", 1.0) as f32, num("--iters", 25.0) as usize, num("--step", 1.0));
    let nocache = args.iter().any(|a| a == "--nocache");
    let opts = TextOpts { gamma: num("--gamma", 1.45) as f32, snap: !args.iter().any(|a| a == "--no-snap") };
    let mode = PaintMode::parse(args);

    let t = Instant::now();
    let mut base: Value = serde_json::from_str(&std::fs::read_to_string(&args[2]).expect("input")).expect("input JSON");
    let read_input_ms = ms(t);
    let font = base["fonts"][0].clone();
    let t = Instant::now();
    let bytes = std::fs::read(FONT_PATH).expect("font");
    let font_read_ms = ms(t);
    let t = Instant::now();
    let digest = format!("{:x}", Sha256::digest(&bytes));
    assert_eq!(digest, font["hash"].as_str().unwrap(), "font bytes do not match the manifest hash");
    let font_hash_ms = ms(t);
    let t = Instant::now();
    let shaper = Shaper::from_bytes(bytes);
    let text = TextPainter::new(&shaper);
    let font_parse_ms = ms(t);
    let t = Instant::now();
    let pool = Pool::new(par::requested(args));
    let pool_ms = ms(t);
    let face = Face { shaper: &shaper, id: font["id"].as_str().unwrap(), hash: font["hash"].as_str().unwrap(), family: font["family"].as_str().unwrap() };
    let image = tiny_skia::Pixmap::new(1, 1).unwrap();
    let painter = Painter { shaper: &shaper, text, image, opts, mode, prof: Default::default() };
    let startup_ms = ms(start);

    // Inputs differ only in viewport width; built outside the timers like the server builds them.
    let w0 = base["viewport"]["width"].as_f64().unwrap();
    let inputs: Vec<(f64, String)> = (0..iters)
        .map(|k| {
            let w = w0 + k as f64 * step;
            base["viewport"]["width"] = json!(w);
            (w, serde_json::to_string(&base).unwrap())
        })
        .collect();
    let (runs, geometry, first_done_ms) = pool.install(|| {
        let mut cache = MeasureCache::new();
        let (mut runs, mut geometry, mut first_done_ms) = (Vec::new(), Map::new(), 0.0);
        for (k, (w, input)) in inputs.iter().enumerate() {
            if nocache {
                cache.clear();
            }
            let out = pipeline::run(&face, &painter, input, &mut cache, scale).expect("bench run");
            if k == 0 {
                first_done_ms = ms(start);
                if let Some(path) = opt(args, "--png") {
                    std::fs::write(path, &out.png).unwrap();
                    std::fs::write(path.replace(".png", "-fast.png"), &out.png_fast).unwrap();
                }
            }
            if opt(args, "--geometry").is_some() {
                geometry.insert(format!("{w}"), crate::convert::geometry(&out.scene));
            }
            runs.push(out.stages);
        }
        (runs, geometry, first_done_ms)
    });
    if let Some(path) = opt(args, "--geometry") {
        std::fs::write(path, Value::Object(geometry).to_string()).unwrap();
    }
    let report = json!({
        "scale": scale, "iters": iters, "step": step, "nocache": nocache,
        "textOpts": {"gamma": opts.gamma, "snap": opts.snap}, "paint": mode.name(), "threads": pool.threads(),
        "startup": {"readInputMs": read_input_ms, "fontReadMs": font_read_ms, "fontHashMs": font_hash_ms,
                    "fontParseMs": font_parse_ms, "poolMs": pool_ms, "totalMs": startup_ms},
        "firstRunDoneMs": first_done_ms,
        "runs": runs,
    });
    match opt(args, "--json") {
        Some(path) => std::fs::write(path, report.to_string()).unwrap(),
        None => println!("{report}"),
    }
}
