use crate::{PaintScene, Raster, RenderOptions, Resources};
use opencanvas_layout::SceneNode;
use serde_json::json;
mod color_management;
mod errors;
mod geometry;
mod images;
mod opacity;
mod sizing;
mod text;

fn node(id: &str, x: f32, y: f32, w: f32, h: f32, fill: &str) -> SceneNode {
    serde_json::from_value(json!({
        "id":id,"parentId":null,"kind":"rectangle","bounds":{"x":x,"y":y,"width":w,"height":h},
        "lines":[],"fontId":null,"fontSize":16,"lineHeight":24,"fill":fill,
        "assetId":null,"fit":"contain","opacity":1,"clip":false,"radius":0,"crop":null
    }))
    .unwrap()
}
fn scene(nodes: Vec<SceneNode>) -> PaintScene {
    let mut s:PaintScene=serde_json::from_value(json!({
        "schemaVersion":1,"coordinateSpace":"frame-local",
        "provenance":{"projectId":"test","revision":1,"frameId":"f","viewport":{"width":64,"height":64},
            "theme":"default","state":"default","sceneVersion":"scene-3","rendererVersion":"web-4",
            "sourceSceneKey":"key","fontManifestHash":"fonts","assetManifestHash":"assets"},
        "nodes":[],"fonts":[],"assets":[],"diagnostics":[]
    })).unwrap();
    s.nodes = nodes;
    s
}
fn render(s: &PaintScene) -> Raster {
    crate::render(s, &Resources::default(), &RenderOptions::default()).unwrap()
}
fn pixel(r: &Raster, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * r.width + x) * 4) as usize;
    r.rgba[at..at + 4].try_into().unwrap()
}
