//! `paint-gate <input.json>... [--scene SCENE --image PNG] [--baseline DIR] [--iters N] [--json PATH]`
//! Pixel gate for the bbox painter, run before any timing is reported:
//! (a) bbox (1 thread) against the original full-canvas painter (`--paint full`), premultiplied RGBA;
//! (b) bbox at 1, 2 and N threads against each other, pixmap and demultiplied RGBA (the PNG input);
//! (c) both checks on the Part B scene (image, border, clip, opacity group), two text options.
//! Inputs run the bench's warm sequence (viewport width + k, k < iters, one measure cache) at scale
//! 1 and 2, text options as `bench`. With --baseline, the k = 0 PNGs `<stem>-s<scale>.png` written by
//! the old binary are decoded and compared too. Exits non-zero on any difference.
use crate::compare::read_png;
use crate::convert;
use crate::encode;
use crate::font::{Shaper, FONT_PATH};
use crate::measure::Face;
use crate::mode::PaintMode;
use crate::paint::Painter;
use crate::par::Pool;
use crate::pipeline::{self, Stages};
use crate::scene::Scene;
use crate::text::{TextOpts, TextPainter};
use opencanvas_layout::MeasureCache;
use serde_json::{json, Value};
use std::path::Path;
use tiny_skia::Pixmap;

const SCALES: [f32; 2] = [1.0, 2.0];

fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).map(|i| args[i + 1].as_str())
}

/// Totals for one input × scale (or scene × options × scale).
#[derive(Default)]
struct Group { cases: u32, full_eq: u32, threads_eq: u32, text_nodes: u32, fallbacks: u32, baseline: Option<bool>, baseline_png: Option<bool>, diffs: Vec<String> }

impl Group {
    fn ok(&self) -> bool {
        self.full_eq == self.cases && self.threads_eq == self.cases && self.baseline != Some(false)
    }
    fn json(&self, name: &str) -> Value {
        json!({"case": name, "cases": self.cases, "fullEqualsBbox": self.full_eq, "threadsEqual": self.threads_eq,
               "textNodes": self.text_nodes, "fallbacks": self.fallbacks, "baselineK0": self.baseline,
               "baselinePngBytesK0": self.baseline_png, "diffs": self.diffs})
    }
}

/// First differing byte as "pixel (x, y) channel c: a vs b", plus the count of differing bytes.
fn diff(a: &[u8], b: &[u8], width: u32) -> Option<String> {
    if a == b { return None; }
    let n = a.iter().zip(b).filter(|(x, y)| x != y).count() + a.len().abs_diff(b.len());
    let i = a.iter().zip(b).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
    let p = (i / 4) as u32;
    Some(format!("{n} bytes differ, first at pixel ({}, {}) channel {}: {:?} vs {:?}", p % width, p / width, i % 4, a.get(i), b.get(i)))
}

struct Gate<'a> { full: Painter<'a>, bbox: Painter<'a>, pools: Vec<Pool> }

impl Gate<'_> {
    /// One scene at one scale; returns the demultiplied RGBA of the bbox painter.
    fn case(&self, g: &mut Group, label: &str, scene: &Scene, scale: f32) -> Vec<u8> {
        g.cases += 1;
        let full = self.pools[0].install(|| self.full.render(scene, scale));
        let mut outs: Vec<(Pixmap, Vec<u8>)> = Vec::new();
        for pool in &self.pools {
            let px = pool.install(|| self.bbox.render(scene, scale));
            if outs.is_empty() {
                let p = self.bbox.prof.lock().unwrap();
                (g.text_nodes, g.fallbacks) = (p.text_nodes, g.fallbacks + p.fallbacks);
            }
            let rgba = pool.install(|| encode::rgba(&px));
            outs.push((px, rgba));
        }
        let (ref one, ref rgba) = outs[0];
        match diff(full.data(), one.data(), full.width()) {
            None => g.full_eq += 1,
            Some(d) => g.diffs.push(format!("{label} (a) full vs bbox: {d}")),
        }
        let mut same = true;
        for ((px, r), pool) in outs.iter().zip(&self.pools).skip(1) {
            for d in [diff(one.data(), px.data(), one.width()), diff(rgba, r, one.width())].into_iter().flatten() {
                g.diffs.push(format!("{label} (b) 1 vs {} threads: {d}", pool.threads()));
                same = false;
            }
        }
        g.threads_eq += same as u32;
        outs.swap_remove(0).1
    }
}

fn painters<'a>(shaper: &'a Shaper, image: &Pixmap, opts: TextOpts) -> (Painter<'a>, Painter<'a>) {
    let make = |mode| Painter { shaper, text: TextPainter::new(shaper), image: image.clone(), opts, mode, prof: Default::default() };
    (make(PaintMode::Full), make(PaintMode::Bbox))
}

pub fn main(args: &[String]) {
    let iters: usize = opt(args, "--iters").map_or(25, |v| v.parse().unwrap());
    let shaper = Shaper::from_bytes(std::fs::read(FONT_PATH).expect("font"));
    // 1, 2 and one per core; a build without `parallel` has a single sequential "pool".
    let mut counts: Vec<usize> = [1, 2, 0].iter().map(|&c| Pool::new(c).threads()).collect();
    counts.dedup();
    let pools: Vec<Pool> = counts.iter().map(|&c| Pool::new(c)).collect();
    println!("paint-gate: threads {counts:?}");
    let mut report = Vec::new();
    let mut ok = true;
    let mut finish = |name: String, g: Group| {
        println!("{name}: {} cases, (a) full = bbox {}/{}, (b) threads equal {}/{}, text nodes {}, fallbacks {}, baseline k=0 {:?} (PNG bytes {:?})",
            g.cases, g.full_eq, g.cases, g.threads_eq, g.cases, g.text_nodes, g.fallbacks, g.baseline, g.baseline_png);
        g.diffs.iter().take(5).for_each(|d| println!("  {d}"));
        ok &= g.ok();
        report.push(g.json(&name));
    };
    let inputs: Vec<&String> = args[2..].iter().take_while(|a| !a.starts_with("--")).collect();
    let empty = Pixmap::new(1, 1).unwrap();
    let opts = TextOpts { gamma: 1.45, snap: true };
    let (full, bbox) = painters(&shaper, &empty, opts);
    let gate = Gate { full, bbox, pools };
    for input in inputs {
        let stem = Path::new(input).file_stem().unwrap().to_string_lossy().to_string();
        let mut base: Value = serde_json::from_str(&std::fs::read_to_string(input).expect("input")).expect("input JSON");
        let font = base["fonts"][0].clone();
        let face = Face { shaper: &shaper, id: font["id"].as_str().unwrap(), hash: font["hash"].as_str().unwrap(), family: font["family"].as_str().unwrap() };
        let w0 = base["viewport"]["width"].as_f64().unwrap();
        let mut cache = MeasureCache::new();
        let mut groups: Vec<Group> = SCALES.iter().map(|_| Group::default()).collect();
        for k in 0..iters {
            base["viewport"]["width"] = json!(w0 + k as f64);
            let scene = pipeline::layout(&face, &base.to_string(), &mut cache, &mut Stages::default()).expect("layout");
            let scene = convert::to_paint(&scene);
            for (g, &scale) in groups.iter_mut().zip(&SCALES) {
                let rgba = gate.case(g, &format!("{stem} s{scale} k={k}"), &scene, scale);
                if let (0, Some(dir)) = (k, opt(args, "--baseline")) {
                    let file = format!("{dir}/{stem}-s{scale}.png");
                    let old = read_png(&file);
                    g.baseline = Some(old.px == rgba);
                    let png = gate.pools[0].install(|| encode::png(&gate.bbox.render(&scene, scale), encode::level("default")));
                    g.baseline_png = Some(std::fs::read(&file).unwrap() == png);
                }
            }
        }
        for (g, scale) in groups.into_iter().zip(SCALES) { finish(format!("{stem} s{scale}"), g); }
    }
    if let (Some(path), Some(image)) = (opt(args, "--scene"), opt(args, "--image")) {
        let scene: Scene = serde_json::from_str(&std::fs::read_to_string(path).expect("scene")).expect("scene JSON");
        let image = Pixmap::load_png(image).expect("image");
        for opts in [TextOpts { gamma: 1.0, snap: false }, TextOpts { gamma: 1.45, snap: true }] {
            let (full, bbox) = painters(&shaper, &image, opts);
            let gate = Gate { full, bbox, pools: counts.iter().map(|&c| Pool::new(c)).collect() };
            for scale in SCALES {
                let mut g = Group::default();
                let name = format!("scene gamma {} snap {} s{scale}", opts.gamma, opts.snap);
                gate.case(&mut g, &name, &scene, scale);
                finish(name, g);
            }
        }
    }
    if let Some(path) = opt(args, "--json") {
        std::fs::write(path, json!({"threads": counts, "iters": iters, "groups": report}).to_string()).unwrap();
    }
    println!("paint-gate: {}", if ok { "PASS (byte-identical everywhere)" } else { "FAIL" });
    if !ok { std::process::exit(1); }
}
