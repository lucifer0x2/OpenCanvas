//! `frames <scene.json> <dir>` (Task 09): the measurement scenes, written once so the native bench
//! (frame_bench.rs) and the wasm host render the same JSON.
//! - spike.json: the Part B scene as is (400×300).
//! - frame.json: 12 copies on a 4×3 grid at a 360×300 pitch, viewport 1440×900. Each copy is 400 wide,
//!   so the right 40 px of a card in columns 1–3 are overdrawn by its neighbour's white root.
//! - tile.json: frame.json moved by (−720, −450) with a 720×450 viewport, the bottom-right quarter.
//!   Every node is still painted (the painter does not cull), like redrawing one tile of the frame.
use serde_json::{json, Value};

const GRID: (usize, usize) = (4, 3);
const PITCH: (f64, f64) = (360.0, 300.0);
const VIEWPORT: (f64, f64) = (1440.0, 900.0);

fn shift(n: &mut Value, dx: f64, dy: f64) {
    let b = &mut n["bounds"];
    b["x"] = json!(b["x"].as_f64().unwrap() + dx);
    b["y"] = json!(b["y"].as_f64().unwrap() + dy);
}

fn with_nodes(base: &Value, nodes: Vec<Value>, (w, h): (f64, f64)) -> Value {
    let mut s = base.clone();
    s["nodes"] = json!(nodes);
    s["provenance"]["viewport"] = json!({"width": w, "height": h});
    s
}

/// `base` tiled GRID times; ids (and parent ids) get a `c<k>-` prefix.
pub fn grid(base: &Value) -> Value {
    let mut nodes = Vec::new();
    for row in 0..GRID.1 {
        for col in 0..GRID.0 {
            let k = row * GRID.0 + col;
            for n in base["nodes"].as_array().unwrap() {
                let mut n = n.clone();
                n["id"] = json!(format!("c{k}-{}", n["id"].as_str().unwrap()));
                if let Some(p) = n["parentId"].as_str() {
                    n["parentId"] = json!(format!("c{k}-{p}"));
                }
                shift(&mut n, col as f64 * PITCH.0, row as f64 * PITCH.1);
                nodes.push(n);
            }
        }
    }
    with_nodes(base, nodes, VIEWPORT)
}

/// The bottom-right quarter of `frame` as its own scene.
pub fn tile(frame: &Value) -> Value {
    let (w, h) = (VIEWPORT.0 / 2.0, VIEWPORT.1 / 2.0);
    let mut nodes = frame["nodes"].as_array().unwrap().clone();
    for n in &mut nodes {
        shift(n, -w, -h);
    }
    with_nodes(frame, nodes, (w, h))
}

pub fn main(args: &[String]) {
    let base: Value = serde_json::from_str(&std::fs::read_to_string(&args[2]).expect("scene")).expect("scene JSON");
    let dir = &args[3];
    std::fs::create_dir_all(dir).unwrap();
    let frame = grid(&base);
    let tile = tile(&frame);
    for (name, scene) in [("spike", &base), ("frame", &frame), ("tile", &tile)] {
        let path = format!("{dir}/{name}.json");
        std::fs::write(&path, scene.to_string()).unwrap();
        println!("{path}: {} nodes", scene["nodes"].as_array().unwrap().len());
    }
}
