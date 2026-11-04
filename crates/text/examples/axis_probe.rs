//! Controlled single-glyph axis probe; not a second text measurement implementation.
use opencanvas_text::Font;
use serde_json::Value;
use std::{error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let path = Path::new("spikes/native-render-parity/out/oracle.json");
    let report: Value = serde_json::from_slice(&fs::read(path)?)?;
    for face in report["fonts"].as_array().ok_or("fonts")? {
        let m = &face["manifest"];
        let bytes = fs::read(
            path.parent()
                .ok_or("parent")?
                .join(face["sourceBytes"].as_str().ok_or("sourceBytes")?),
        )?;
        let font = Font::from_bytes(bytes, m["hash"].as_str().ok_or("hash")?)?;
        let inner = font.shaping_font();
        println!(
            "{} axes: {:?}",
            m["id"],
            inner.axes().iter().collect::<Vec<_>>()
        );
        if m["id"] == "inter" {
            let run = font.positioned_run("ς i\u{307}", 16.0, 0.0)?;
            let instance = font.shaping_font_at_size(16.0)?;
            for glyph in &run.glyphs {
                println!(
                    "{glyph:?} ink={:?}",
                    instance.glyph_metrics().extents(glyph.id)
                );
            }
        }
        for size in [12.0, 16.0, 17.5, 24.0, 32.0] {
            for opsz in [None, Some(size), Some(size * 0.75)] {
                let instance = opsz.map_or_else(
                    || inner.clone(),
                    |opsz| {
                        inner
                            .instance_builder()
                            .variations([("opsz", opsz)])
                            .build()
                    },
                );
                let widths: Vec<_> = "Aeo "
                    .chars()
                    .map(|c| {
                        let id = instance.charmap().map_unicode(c as u32).unwrap_or_default();
                        (
                            c,
                            instance.glyph_metrics().h_advance(id) as f64 * size as f64
                                / instance.units_per_em() as f64,
                        )
                    })
                    .collect();
                println!(
                    "size={size} opsz={opsz:?} coords={:?} {widths:?}",
                    instance.normalized_coords()
                );
            }
        }
    }
    Ok(())
}
