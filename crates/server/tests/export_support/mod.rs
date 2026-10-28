//! Fixture design and PNG pixel decoding for `export_nodes` tests.
use serde_json::{json, Value};

/// One free frame with a red box, fractional-width text, a clipped and faded group, a
/// zero-width line and a node too large to export at 4x.
pub fn design() -> Value {
    let rect = |id: &str, parent: &str, props: Value| json!({"op":"create_node","id":id,"kind":"rectangle","parentId":parent,"props":props});
    json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{"name":"Home"}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"name":"Screen","width":320,"height":200,"fill":"#ffffff","layout":{"mode":"free"}}},
        rect("box", "frame", json!({"name":"Box","x":10,"y":10,"width":40,"height":40,"radius":6,"fill":"#ef4444"})),
        {"op":"create_node","id":"label","kind":"text","parentId":"frame","props":{"x":10,"y":60,"text":"Export me","width":133.5,"height":"hug","fontId":"font_bundled","fontSize":20,"lineHeight":28}},
        {"op":"create_node","id":"clipper","kind":"group","parentId":"frame","props":{"x":200,"y":10,"width":30,"height":60,"clip":true,"opacity":0.5,"layout":{"mode":"free"}}},
        rect("inner", "clipper", json!({"x":0,"y":0,"width":60,"height":60,"fill":"#2563eb"})),
        rect("thin", "frame", json!({"x":10,"y":120,"width":0,"height":20,"fill":"#000000"})),
        rect("huge", "frame", json!({"x":0,"y":0,"width":2100,"height":2100,"fill":"#00000000"})),
    ])
}

/// RGBA pixels and size of a downloaded PNG.
pub fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    (info.width, info.height, buf)
}

pub fn pixel(image: &(u32, u32, Vec<u8>), x: u32, y: u32) -> [u8; 4] {
    let at = ((y * image.0 + x) * 4) as usize;
    image.2[at..at + 4].try_into().unwrap()
}
