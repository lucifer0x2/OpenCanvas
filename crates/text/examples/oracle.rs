//! Filesystem I/O belongs only to this dev adapter, never the runtime library.
use opencanvas_text::{transform_text, Font, FontSet, MeasureRequest};
use serde_json::{json, Value};
use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("spikes/native-render-parity/out/oracle.json"));
    let output = std::env::args()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_file_name("native-measurements.json"));
    let bytes = fs::read(&input)?;
    let report: Value = serde_json::from_slice(&bytes)?;
    if report["schemaVersion"] != 1 || report["kind"] != "chromium-measurement-oracle-only" {
        return Err("unsupported oracle schema".into());
    }
    let mut fonts = FontSet::new();
    for face in report["fonts"].as_array().ok_or("fonts array")? {
        let m = &face["manifest"];
        let source = face["sourceBytes"].as_str().ok_or("sourceBytes")?;
        let relative = PathBuf::from(source);
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("font source must be report-relative without traversal".into());
        }
        let data = fs::read(input.parent().ok_or("report parent")?.join(relative))?;
        let font = Font::from_bytes(data, m["hash"].as_str().ok_or("hash")?)?.with_face(
            m["family"].as_str().ok_or("family")?,
            m["weight"].as_u64().ok_or("weight")? as u16,
            m["style"].as_str().ok_or("style")?,
        )?;
        fonts.insert(m["id"].as_str().ok_or("id")?, font)?;
    }
    let mut measurements = serde_json::Map::new();
    let mut probes = serde_json::Map::new();
    let mut errors = serde_json::Map::new();
    for value in report["requests"].as_array().ok_or("requests array")? {
        let request: MeasureRequest = serde_json::from_value(value.clone())?;
        match fonts.measure(&request) {
            Ok(measurement) => {
                let font = fonts.get(&request.font_id).ok_or("font ID")?;
                let text = transform_text(&request.text, request.text_transform.as_deref())?;
                let spacing = request.letter_spacing.unwrap_or_default();
                let advance = font
                    .positioned_run(&text, request.font_size, spacing)?
                    .advance;
                let lines: Vec<_> = measurement
                    .lines
                    .iter()
                    .map(|line| {
                        font.positioned_run(&line.text, request.font_size, spacing)
                            .map(|run| json!({"text":line.text,"advance":run.advance}))
                    })
                    .collect::<Result<_, _>>()?;
                probes.insert(
                    request.key.clone(),
                    json!({
                        "transformedText":text,"advance":advance,"lines":lines,
                        "metrics": {"ascent":font.metrics(request.font_size)?.ascent,
                            "descent":font.metrics(request.font_size)?.descent}
                    }),
                );
                // to_value promotes f32 to its exact f64 value. This avoids shortest-f32
                // JSON decimal rendering pretending a JS Number equals the original f64.
                measurements.insert(request.key, serde_json::to_value(measurement)?);
            }
            Err(error) => {
                errors.insert(
                    request.key,
                    json!({"code":error.code(),
                "message":error.to_string()}),
                );
            }
        }
    }
    use sha2::{Digest, Sha256};
    let mut source_hashes = serde_json::Map::new();
    for name in [
        "Cargo.toml",
        "src/font.rs",
        "src/shaping.rs",
        "src/itemize.rs",
        "src/optical.rs",
        "src/measure.rs",
        "src/metrics.rs",
        "src/wrap.rs",
        "src/transform.rs",
        "src/coverage.rs",
        "src/decode.rs",
        "examples/oracle.rs",
    ] {
        let source = fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name))?;
        source_hashes.insert(
            name.to_owned(),
            json!(format!("{:x}", Sha256::digest(source))),
        );
    }
    let result = json!({"schemaVersion":1,"kind":"native-text-measurements",
        "oracleSha256":format!("{:x}",Sha256::digest(&bytes)),
        "sourceHashes":source_hashes,
        "environment":{"os":std::env::consts::OS,"arch":std::env::consts::ARCH},
        "measurements":measurements,"probes":probes,"errors":errors});
    fs::write(output, serde_json::to_vec_pretty(&result)?)?;
    Ok(())
}
