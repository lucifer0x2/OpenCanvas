//! Collect actual outline commands, independently of measured advances.
use opencanvas_text::{DrawMemory, Font, PathSink, ScalerFont};

#[derive(Default)]
struct Commands(Vec<u32>);
impl Commands {
    fn add(&mut self, command: u32, values: &[f32]) {
        self.0.push(command);
        self.0.extend(values.iter().map(|v| v.to_bits()));
    }
}
impl PathSink for Commands {
    fn move_to(&mut self, x: f32, y: f32) {
        self.add(0, &[x, y]);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.add(1, &[x, y]);
    }
    fn quad_to(&mut self, a: f32, b: f32, x: f32, y: f32) {
        self.add(2, &[a, b, x, y]);
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        self.add(3, &[a, b, c, d, x, y]);
    }
    fn close(&mut self) {
        self.add(4, &[]);
    }
}

pub fn outlines(font: &Font, size: f32, text: &str) -> Vec<Vec<u32>> {
    let instance = font.shaping_font_at_size(size).unwrap();
    let scaler = ScalerFont::unhinted(instance, None);
    font.positioned_run(text, size, 0.0)
        .unwrap()
        .glyphs
        .iter()
        .map(|g| {
            let mut commands = Commands::default();
            scaler
                .outline(g.id)
                .unwrap()
                .draw(DrawMemory::Scratch, &mut commands)
                .unwrap();
            assert!(!commands.0.is_empty());
            commands.0
        })
        .collect()
}
