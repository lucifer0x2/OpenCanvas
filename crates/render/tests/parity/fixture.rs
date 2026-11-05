//! Same source is built as a native executable and a test-only WASM cdylib.
use opencanvas_render::{
    encode_png, render, PaintScene, Raster, RenderOptions, ResourceBytes, Resources,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const FONT: &[u8] = include_bytes!("../../../../renderer/fonts/NotoSans.ttf");
fn node(id: &str, x: f32, y: f32, w: f32, h: f32, fill: Value) -> Value {
    json!({"id":id,"parentId":null,"kind":"rectangle","bounds":{"x":x,"y":y,"width":w,"height":h},
        "lines":[],"fontId":null,"fontSize":16,"lineHeight":24,"fill":fill,
        "assetId":null,"fit":"contain","opacity":1,"clip":false,"radius":0,"crop":null})
}
fn scene(nodes: Vec<Value>) -> PaintScene {
    serde_json::from_value(json!({"schemaVersion":1,"coordinateSpace":"frame-local",
        "provenance":{"projectId":"fixture","revision":1,"frameId":"f","viewport":{"width":96,"height":80},
            "theme":"default","state":"default","sceneVersion":"scene-3","rendererVersion":"web-4",
            "sourceSceneKey":"fixture","fontManifestHash":"fixture","assetManifestHash":"fixture"},
        "nodes":nodes,"fonts":[],"assets":[],"diagnostics":[]})).unwrap()
}
fn fixture(case: u32) -> Raster {
    let mut p = node("p", 10.25, 8.5, 70.0, 60.0, json!("#abcdef"));
    let mut child = node("c", 20.5, 12.25, 55.0, 42.0, json!("#123456"));
    child["parentId"] = json!("p");
    match case {
        0 => {
            p["radius"] = json!([25, 4, 16, 8]);
            p["clip"] = json!(true);
            p["stroke"] =
                json!({"align":"outside","width":[3,7,5,2],"color":["#f00","#0f08","#00f","#ff0"]});
            child["fill"] = json!({"type":"radial","radius":[34,18],"center":[0.4,0.5],"stops":[[0,"#ff000040"],[0.45,"#0000ff90"],[0.45,"#fff"],[1,"#0000"]]});
        }
        1 => {
            p["opacity"] = json!(0.65);
            p["radius"] = json!(12);
            p["shadow"] = json!([
                {"x":4,"y":3,"spread":2,"blur":8,"color":"#0008"},
                {"x":-2,"y":1,"spread":3,"blur":5,"color":"#f00a","inset":true}]);
            child["opacity"] = json!(0.4);
            child["stroke"] =
                json!({"align":"center","width":[3,3,3,3],"color":["#000","#000","#000","#000"]});
        }
        2 => {
            child["kind"] = json!("text");
            child["fontId"] = json!("font");
            child["letterSpacing"] = json!(-0.4);
            child["opacity"] = json!(0.6);
            child["decoration"] = json!({"line":"underline","offset":1.3,"thickness":1.5});
            child["lines"] = json!([
                {"text":"Việt AV","x":0,"y":18,"width":52},
                {"text":"nghiêng","x":1.25,"y":36,"width":54}]);
        }
        _ => {
            child["kind"] = json!("image");
            child["assetId"] = json!("image");
            child["radius"] = json!([9, 15, 4, 11]);
            child["fit"] = json!("cover");
            child["crop"] = json!({"x":0.25,"y":0,"width":0.75,"height":1});
        }
    }
    let mut s = scene(vec![p, child]);
    let font_hash = format!("{:x}", Sha256::digest(FONT));
    let mut resources = Resources::default();
    if case == 2 {
        s.fonts.push(serde_json::from_value(json!({"id":"font","hash":font_hash,"family":"Fixture","weight":400,"style":"normal","url":"unused"})).unwrap());
        resources.fonts.insert(
            "font".into(),
            ResourceBytes {
                hash: &font_hash,
                bytes: FONT,
            },
        );
    }
    let image = png_image();
    let image_hash = format!("{:x}", Sha256::digest(&image));
    if case == 3 {
        s.assets.push(
            serde_json::from_value(
                json!({"id":"image","hash":image_hash,"mediaType":"image/png","url":"unused"}),
            )
            .unwrap(),
        );
        resources.assets.insert(
            "image".into(),
            ResourceBytes {
                hash: &image_hash,
                bytes: &image,
            },
        );
    }
    let raster = render(
        &s,
        &resources,
        &RenderOptions {
            scale: 1.5,
            ..RenderOptions::default()
        },
    )
    .unwrap();
    assert!(!encode_png(&raster).unwrap().is_empty());
    raster
}
fn png_image() -> Vec<u8> {
    let mut bytes = Vec::new();
    let profile = moxcms::ColorProfile::new_srgb().encode().unwrap();
    let mut info = png::Info::with_size(2, 2);
    info.color_type = png::ColorType::Rgba;
    info.bit_depth = png::BitDepth::Eight;
    info.icc_profile = Some(std::borrow::Cow::Owned(profile));
    let mut encoder = png::Encoder::with_info(&mut bytes, info).unwrap();
    encoder.set_compression(png::Compression::Fast);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[
            255, 0, 0, 128, 0, 0, 255, 255, 0, 255, 0, 255, 255, 255, 255, 64,
        ])
        .unwrap();
    bytes
}
fn all_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    for case in 0..4 {
        let r = fixture(case);
        for v in [
            r.width,
            r.height,
            r.draw.opacity_layers as u32,
            r.draw.opacity_layer_fallbacks as u32,
        ] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes.extend(r.rgba);
    }
    bytes
}
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn fixture_rgba() -> u64 {
    let bytes = all_bytes().into_boxed_slice();
    let len = bytes.len() as u32;
    let ptr = Box::leak(bytes).as_ptr() as u32;
    (u64::from(ptr) << 32) | u64::from(len)
}
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn main() {
    use std::io::Write;
    std::io::stdout().write_all(&all_bytes()).unwrap();
}
