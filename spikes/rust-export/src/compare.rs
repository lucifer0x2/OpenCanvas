//! RGBA diff stats (overall and per region) and a chrome | rust | diff×8 side-by-side PNG.
use serde::Deserialize;
use serde_json::{json, Value};

pub struct Rgba { pub w: u32, pub h: u32, pub px: Vec<u8> }

pub fn read_png(path: &str) -> Rgba {
    let mut d = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).expect(path)));
    d.set_transformations(png::Transformations::EXPAND);
    let mut r = d.read_info().unwrap();
    let mut buf = vec![0; r.output_buffer_size().unwrap()];
    let info = r.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba, "{path}");
    buf.truncate(info.buffer_size());
    Rgba { w: info.width, h: info.height, px: buf }
}

pub fn write_png(path: &str, w: u32, h: u32, px: &[u8]) {
    let f = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let mut e = png::Encoder::new(f, w, h);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(px).unwrap();
}

#[derive(Deserialize)]
pub struct Region { pub label: String, pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

#[derive(Default, Clone)]
struct Stat { n: u64, gt0: u64, gt8: u64, gt32: u64, max: u8, sum: u64 }

fn stat_json(s: &Stat) -> Value {
    json!({"pixels": s.n, "gt0": s.gt0, "gt8": s.gt8, "gt32": s.gt32, "maxDelta": s.max,
           "meanDelta": if s.n > 0 { s.sum as f64 / s.n as f64 } else { 0.0 }})
}

pub fn compare(a: &Rgba, b: &Rgba, regions: &[Region], scale: f32, side: Option<&str>) -> Value {
    assert_eq!((a.w, a.h), (b.w, b.h));
    let labels = ["text", "image", "shapes"];
    let mut stats = vec![Stat::default(); 4];
    let mut diff = vec![0u8; a.px.len()];
    for y in 0..a.h {
        for x in 0..a.w {
            let i = ((y * a.w + x) * 4) as usize;
            let d = (0..4).map(|c| a.px[i + c].abs_diff(b.px[i + c])).max().unwrap();
            let (sx, sy) = ((x as f32 + 0.5) / scale, (y as f32 + 0.5) / scale);
            let region = regions.iter().find(|r| sx >= r.x && sx < r.x + r.w && sy >= r.y && sy < r.y + r.h)
                .map(|r| labels.iter().position(|l| *l == r.label).unwrap()).unwrap_or(2);
            for k in [3, region] {
                let s = &mut stats[k];
                s.n += 1; s.sum += d as u64; s.max = s.max.max(d);
                if d > 0 { s.gt0 += 1 } if d > 8 { s.gt8 += 1 } if d > 32 { s.gt32 += 1 }
            }
            let v = 255 - (d as u32 * 8).min(255) as u8;
            diff[i..i + 4].copy_from_slice(&[255, v, v, 255]);
        }
    }
    if let Some(path) = side {
        let (w, h) = (a.w * 3 + 8, a.h);
        let mut out = vec![128u8; (w * h * 4) as usize];
        for y in 0..h {
            for (k, src) in [&a.px, &b.px, &diff].iter().enumerate() {
                let x0 = k as u32 * (a.w + 4);
                for x in 0..a.w {
                    let s = ((y * a.w + x) * 4) as usize;
                    let o = ((y * w + x0 + x) * 4) as usize;
                    // Composite over white so transparent pixels read as the page background.
                    let al = src[s + 3] as u32;
                    for c in 0..3 { out[o + c] = ((src[s + c] as u32 * al + 255 * (255 - al)) / 255) as u8; }
                    out[o + 3] = 255;
                }
            }
        }
        write_png(path, w, h, &out);
    }
    json!({"all": stat_json(&stats[3]), "text": stat_json(&stats[0]), "image": stat_json(&stats[1]), "shapes": stat_json(&stats[2])})
}
