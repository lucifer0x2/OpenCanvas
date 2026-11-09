//! Native vs WASM premultiplied RGBA (tiny-skia pixmap bytes): pixels with any channel Δ > 0, max Δ
//! per channel, a Δ histogram, the bounding box of the differing pixels and the first one.
use serde_json::{json, Value};

pub fn compare(native: &[u8], wasm: &[u8], width: u32) -> Value {
    assert_eq!(native.len(), wasm.len(), "pixmap sizes differ");
    let w = width as usize;
    let mut max = [0u8; 4];
    // Max channel Δ of a pixel: 1, 2, 3–8, > 8.
    let mut hist = [0u64; 4];
    let (mut differ, mut first) = (0u64, Value::Null);
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for (i, (a, b)) in native.chunks_exact(4).zip(wasm.chunks_exact(4)).enumerate() {
        let mut d = 0u8;
        for c in 0..4 {
            let dc = a[c].abs_diff(b[c]);
            max[c] = max[c].max(dc);
            d = d.max(dc);
        }
        if d == 0 {
            continue;
        }
        let (x, y) = (i % w, i / w);
        if differ == 0 {
            first = json!({"x": x, "y": y, "native": a, "wasm": b});
        }
        differ += 1;
        hist[match d { 1 => 0, 2 => 1, 3..=8 => 2, _ => 3 }] += 1;
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
    }
    let bbox = if differ > 0 { json!([x0, y0, x1 + 1, y1 + 1]) } else { Value::Null };
    json!({
        "pixels": native.len() / 4, "pixelsDiffer": differ, "maxDelta": max.iter().max(), "maxDeltaRgba": max,
        "hist": {"d1": hist[0], "d2": hist[1], "d3to8": hist[2], "gt8": hist[3]}, "bbox": bbox, "first": first,
    })
}
