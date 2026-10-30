//! One export job in process (Part D): parse the server's layout input, run `opencanvas_layout`
//! passes answered by native measurement, paint with the spike painter, encode PNG. One timer
//! per stage; nothing here touches the network, Node or Chromium.
use crate::convert;
use crate::encode;
use crate::measure::{self, Face};
use crate::paint::Painter;
use crate::profile::PaintProfile;
use opencanvas_layout::{compute, LayoutInput, LayoutStep, MeasureCache, Scene, MAX_PASSES};
use serde::Serialize;
use std::time::Instant;

#[derive(Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stages {
    /// JSON text → `LayoutInput` (layout-step does the same once per pass on the Chrome path).
    pub parse_ms: f64,
    /// Every `compute` call, including the final scene build and its SHA-256 scene keys.
    pub layout_ms: f64,
    pub passes: u32,
    /// HarfRust measurement of every pending request (`measureBatch` equivalent).
    pub measure_ms: f64,
    pub measured: usize,
    /// Scene conversion + tiny-skia paint into an RGBA pixmap.
    pub paint_ms: f64,
    pub encode_ms: f64,
    pub encode_fast_ms: f64,
    pub png_bytes: usize,
    pub png_fast_bytes: usize,
    /// parse + layout + measure + paint + encode at the png crate default (Balanced).
    pub total_ms: f64,
    /// Same with `Compression::Fast`.
    pub total_fast_ms: f64,
    /// Process CPU time (user + sys, every thread) over what `total_ms` covers; 0 off unix.
    pub cpu_ms: f64,
    pub profile: PaintProfile,
}

pub struct Output {
    pub stages: Stages,
    pub scene: Scene,
    pub png: Vec<u8>,
    pub png_fast: Vec<u8>,
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

/// CPU time of this process so far (user + sys, all threads), ms.
#[cfg(unix)]
fn cpu_ms() -> f64 {
    let mut u = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: getrusage only writes the struct it is given.
    let u = unsafe {
        libc::getrusage(libc::RUSAGE_SELF, u.as_mut_ptr());
        u.assume_init()
    };
    let tv = |t: libc::timeval| t.tv_sec as f64 * 1e3 + t.tv_usec as f64 / 1e3;
    tv(u.ru_utime) + tv(u.ru_stime)
}

#[cfg(not(unix))]
fn cpu_ms() -> f64 {
    0.0
}

/// Parse + layout passes answered by native measurement; fills the parse/layout/measure stages.
pub fn layout(face: &Face, input_json: &str, cache: &mut MeasureCache, s: &mut Stages) -> Result<Scene, String> {
    let t = Instant::now();
    let input: LayoutInput = serde_json::from_str(input_json).map_err(|e| e.to_string())?;
    s.parse_ms = ms(t);
    for _ in 0..MAX_PASSES {
        s.passes += 1;
        let t = Instant::now();
        let step = compute(&input, cache).map_err(|e| e.to_string())?;
        s.layout_ms += ms(t);
        match step {
            LayoutStep::Ready { scene } => return Ok(scene),
            LayoutStep::Pending { requests } => {
                let t = Instant::now();
                s.measured += measure::measure_batch(face, &requests, cache)?;
                s.measure_ms += ms(t);
            }
        }
    }
    Err("LAYOUT_NON_CONVERGENCE".into())
}

pub fn run(face: &Face, painter: &Painter, input_json: &str, cache: &mut MeasureCache, scale: f32) -> Result<Output, String> {
    let mut s = Stages::default();
    let cpu = cpu_ms();
    let scene = layout(face, input_json, cache, &mut s)?;
    let t = Instant::now();
    let px = painter.render(&convert::to_paint(&scene), scale);
    s.paint_ms = ms(t);
    s.profile = *painter.prof.lock().unwrap();
    let t = Instant::now();
    let png = encode::png(&px, encode::level("default"));
    s.encode_ms = ms(t);
    s.cpu_ms = cpu_ms() - cpu;
    let t = Instant::now();
    let png_fast = encode::png(&px, encode::level("fast"));
    s.encode_fast_ms = ms(t);
    (s.png_bytes, s.png_fast_bytes) = (png.len(), png_fast.len());
    let before = s.parse_ms + s.layout_ms + s.measure_ms + s.paint_ms;
    (s.total_ms, s.total_fast_ms) = (before + s.encode_ms, before + s.encode_fast_ms);
    Ok(Output { stages: s, scene, png, png_fast })
}
