//! In-process stand-in for renderer/measurement.ts `measureBatch`: answers the layout crate's
//! `Pending.requests` with HarfRust widths in the Chrome advance model (font.rs `width_fixed`
//! variant 2) and the wrap port (wrap.rs), filling the same `Measurement`/`TextLine` fields.
use crate::font::Shaper;
use crate::wrap;
use opencanvas_layout::{MeasureCache, MeasureRequest, Measurement, TextLine};

/// The value a JS caller sees for an f32 that crossed JSON: shortest decimal, parsed as f64.
/// `measureBatch` compares widths against this f64, so the port must too.
fn js(v: f32) -> f64 {
    format!("{v}").parse().unwrap()
}

/// The one face the spike loads, checked like `measureBatch` checks the manifest.
pub struct Face<'a> {
    pub shaper: &'a Shaper,
    pub id: &'a str,
    pub hash: &'a str,
    pub family: &'a str,
}

/// Measures every request into `cache`; returns the number of items measured.
pub fn measure_batch(face: &Face, requests: &[MeasureRequest], cache: &mut MeasureCache) -> Result<usize, String> {
    for r in requests {
        if r.font_id != face.id || r.font_hash != face.hash || r.family != face.family || r.weight != 400 || r.style != "normal" {
            return Err(format!("FONT_MISSING: exact font {} is not ready", r.font_id));
        }
        let (size, lh) = (js(r.font_size), js(r.line_height));
        let px = |t: &str| face.shaper.width_fixed(t, size, 2);
        let lines = wrap::measure(&px, &r.text, size, lh, r.width.map(js));
        let width = lines.iter().map(|l| l.width).fold(0.0, f64::max);
        let height = wrap::round64(lines.len() as f64 * lh);
        let lines = lines
            .into_iter()
            .map(|l| TextLine { text: l.text, x: 0.0, y: l.y as f32, width: l.width as f32 })
            .collect();
        cache.insert(r.key.clone(), Measurement { width: width as f32, height: height as f32, lines });
    }
    Ok(requests.len())
}
