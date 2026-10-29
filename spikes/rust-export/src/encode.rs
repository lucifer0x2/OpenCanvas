//! PNG encode of a premultiplied tiny-skia pixmap at a chosen `png` compression level.
//! Same demultiply as `Pixmap::encode_png` (which only offers the crate default, Balanced), done in
//! parallel pieces (par.rs); compression itself stays single-threaded in the `png` crate.
use crate::par;
use png::Compression;
use tiny_skia::Pixmap;

pub fn level(name: &str) -> Compression {
    match name {
        "fast" => Compression::Fast,
        "fastest" => Compression::Fastest,
        "high" => Compression::High,
        _ => Compression::Balanced,
    }
}

/// Straight (demultiplied) RGBA bytes of `px`.
pub fn rgba(px: &Pixmap) -> Vec<u8> {
    let mut out = vec![0u8; px.data().len()];
    par::zip_chunks(&mut out, par::CHUNK, px.pixels(), par::CHUNK / 4, |dst, src| {
        for (d, p) in dst.chunks_exact_mut(4).zip(src) {
            let c = p.demultiply();
            d.copy_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
        }
    });
    out
}

pub fn png(px: &Pixmap, compression: Compression) -> Vec<u8> {
    let rgba = rgba(px);
    let mut out = Vec::new();
    let mut e = png::Encoder::new(&mut out, px.width(), px.height());
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.set_compression(compression);
    e.write_header().unwrap().write_image_data(&rgba).unwrap();
    out
}
