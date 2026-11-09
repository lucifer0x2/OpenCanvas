//! Deliberately synthetic metrics: an engine oracle, not browser/font parity.
use opencanvas_layout::*;
use serde_json::json;

pub fn oracle(request: &MeasureRequest) -> Measurement {
    let advance = request.font_size / 2.0 + request.letter_spacing.unwrap_or(0.0);
    let capacity = request
        .width
        .map(|w| (w / advance).floor().max(1.0) as usize)
        .unwrap_or(usize::MAX);
    let text = match request.text_transform.as_deref() {
        Some("uppercase") => request.text.to_uppercase(),
        _ => request.text.clone(),
    };
    let chars: Vec<_> = text.chars().collect();
    let lines: Vec<_> = chars
        .chunks(capacity)
        .enumerate()
        .map(|(i, chunk)| TextLine {
            text: chunk.iter().collect(),
            x: 0.0,
            y: i as f32 * request.line_height + 13.0,
            width: (chunk.len() as f32 * advance * 64.0).ceil() / 64.0,
            ink_bounds: Some(Bounds {
                x: -0.5,
                y: -12.0,
                width: chunk.len() as f32 * advance,
                height: 14.0,
            }),
        })
        .collect();
    Measurement {
        width: lines.iter().map(|line| line.width).fold(0.0, f32::max),
        height: lines.len() as f32 * request.line_height,
        lines,
        ascent: 12.0,
        missing: vec![0x2318],
        coverage_unavailable: true,
    }
}

#[derive(Default)]
pub struct RecordingMeasurer(pub Vec<MeasureRequest>);

impl TextMeasurer for RecordingMeasurer {
    fn measure(&mut self, request: &MeasureRequest) -> Result<Measurement, LayoutError> {
        self.0.push(request.clone());
        Ok(oracle(request))
    }
}

pub fn cached_scene(input: &LayoutInput) -> Scene {
    let mut cache = MeasureCache::new();
    for _ in 0..MAX_PASSES {
        match compute(input, &cache).unwrap() {
            LayoutStep::Ready { scene } => return scene,
            LayoutStep::Pending { requests } => {
                assert!(!requests.is_empty());
                for request in requests {
                    cache.insert(request.key.clone(), oracle(&request));
                }
            }
        }
    }
    panic!("synthetic fixture did not converge within the cache-path budget");
}

pub fn fixture(width: f64) -> LayoutInput {
    let mut input: LayoutInput = serde_json::from_value(json!({
        "schemaVersion":1,"projectId":"p","revision":7,"frameId":"f",
        "viewport":{"width":width,"height":800},"theme":"dark","state":"hover",
        "nodes":[
            {"id":"f","kind":"frame","props":{"layout":{
                "mode":"column","padding":7.003,"gap":8,"breakpoints":[
                    {"maxWidth":400,"layout":{"mode":"column","padding":8,"gap":4}}
                ]}}},
            {"id":"row","parentId":"f","kind":"group","props":{
                "width":"fill","height":"hug","layout":{"mode":"row","wrap":true,"gap":12}}},
            {"id":"nowrap","parentId":"f","kind":"text","props":{
                "text":"short","width":"hug","height":"hug","textWrap":"nowrap",
                "textDecoration":"line-through"}}
        ],
        "fonts":[{"id":"font_bundled","family":"Synthetic","hash":"h","url":"/font.ttf"}],
        "assets":[{"id":"a","hash":"asset-hash","url":"/asset","mediaType":"image/png"}]
    }))
    .unwrap();
    for i in 0..3 {
        let card = format!("card{i}");
        input.nodes.push(
            serde_json::from_value(json!({
            "id":card,"parentId":"row","kind":"group","props":{
                "width":"fill","minWidth":120,"height":"hug",
                "layout":{"mode":"column","padding":5,"gap":2}}}))
            .unwrap(),
        );
        for j in 0..2 {
            input.nodes.push(
                serde_json::from_value(json!({
                    "id":format!("text{i}-{j}"),"parentId":card,"kind":"text","props":{
                        "text":"Tiếng Việt long synthetic line ⌘ repeated for wrapping",
                        "width":"fill","height":"hug","fontSize":16,"lineHeight":24,
                        "textAlign":"center","textDecoration":"underline",
                        "letterSpacing":0.25,"textTransform":"uppercase","effectiveTheme":"scoped"}
                }))
                .unwrap(),
            );
        }
        input.nodes.push(
            serde_json::from_value(json!({
                "id":format!("shape{i}"),"parentId":card,"kind":"rectangle",
                "props":{"width":"50%","height":20,"fill":"#123456","radius":3}
            }))
            .unwrap(),
        );
    }
    input
}

pub fn fixed_text() -> LayoutInput {
    let mut input = fixture(300.0);
    input.nodes.truncate(1);
    input.nodes[0].props.layout = None;
    input.nodes.push(
        serde_json::from_value(json!({
            "id":"fixed","parentId":"f","kind":"text","props":{
                "text":"fixed","width":37.337,"height":24,"textAlign":"right"}
        }))
        .unwrap(),
    );
    input
}
