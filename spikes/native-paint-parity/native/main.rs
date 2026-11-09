//! Fixture-only I/O adapter. All paint/decode/font logic remains in the public crate.
use opencanvas_render::{encode_png, render, PaintScene, RenderOptions, ResourceBytes, Resources};
use serde_json::{json, Value};
use std::{collections::BTreeMap, env, error::Error, fs, path::Path};

fn run() -> Result<Value, Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err("usage: native-paint-parity-adapter OUT_ROOT REQUEST_JSON PNG_OUTPUT".into());
    }
    let root = fs::canonicalize(&args[1])?;
    let request_path = fs::canonicalize(&args[2])?;
    if !request_path.starts_with(&root) {
        return Err("request outside fixture root".into());
    }
    let request: Value = serde_json::from_slice(&fs::read(request_path)?)?;
    let scene: PaintScene = serde_json::from_value(request["scene"].clone())?;
    let mut owned = BTreeMap::new();
    for manifest in scene
        .fonts
        .iter()
        .map(|f| (&f.id, &f.hash, &f.url, true))
        .chain(scene.assets.iter().map(|a| (&a.id, &a.hash, &a.url, false)))
    {
        let (id, hash, url, font) = manifest;
        let name = url
            .strip_prefix("/resources/")
            .ok_or("fixture URL required")?;
        if name.is_empty() || name.contains('/') || name.contains('\\') || name == ".." {
            return Err("invalid fixture resource name".into());
        }
        let path = root.join("resources").join(name);
        if !path.exists() {
            continue;
        } // Public strict readiness owns missing-resource errors.
        let path = fs::canonicalize(path)?;
        if !path.starts_with(root.join("resources")) {
            return Err("resource escaped root".into());
        }
        owned.insert((font, id.clone()), (hash.clone(), fs::read(path)?));
    }
    let mut resources = Resources::default();
    for ((font, id), (hash, bytes)) in &owned {
        let entry = ResourceBytes { hash, bytes };
        if *font {
            resources.fonts.insert(id.clone(), entry);
        } else {
            resources.assets.insert(id.clone(), entry);
        }
    }
    let options = RenderOptions {
        scale: serde_json::from_value(request["scale"].clone())?,
        strict: true,
        ..RenderOptions::default()
    };
    let raster = match render(&scene, &resources, &options) {
        Ok(raster) => raster,
        Err(error) => {
            return Ok(json!({"ok": false, "code": error.code, "message": error.message}))
        }
    };
    let output = Path::new(&args[3]);
    let parent = fs::canonicalize(output.parent().ok_or("output parent required")?)?;
    if !parent.starts_with(&root) || output.symlink_metadata().is_ok() {
        return Err("output must be a new file inside fixture root".into());
    }
    fs::write(output, encode_png(&raster)?)?;
    Ok(
        json!({"ok": true, "width": raster.width, "height": raster.height,
        "draw": raster.draw, "diagnostics": raster.diagnostics, "readiness": raster.readiness,
        "sourceSceneKey": raster.source_scene_key}),
    )
}
fn main() {
    match run() {
        Ok(value) => println!("{value}"),
        Err(error) => {
            println!(
                "{}",
                json!({"ok": false, "code": "HARNESS_ERROR", "message": error.to_string()})
            );
            std::process::exit(1);
        }
    }
}
