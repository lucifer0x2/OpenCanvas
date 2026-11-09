//! Own minimal 3x5 bitmap alphabet: an honest recognizable label, never an OS font.
use crate::{
    canvas::Canvas,
    color,
    geometry::{self, Rect},
    RenderError,
};
use opencanvas_layout::SceneNode;
use tiny_skia::FillRule;

fn bits(c: char) -> [u8; 5] {
    match c {
        'M' => [5, 7, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'S' => [7, 4, 7, 1, 7],
        'N' => [5, 7, 7, 7, 5],
        'G' => [7, 4, 5, 5, 7],
        'F' => [7, 4, 6, 4, 4],
        'O' => [7, 5, 5, 5, 7],
        'T' => [7, 2, 2, 2, 2],
        'A' => [2, 5, 7, 5, 5],
        'E' => [7, 4, 6, 4, 7],
        _ => [0; 5],
    }
}
pub(crate) fn paint(c: &mut Canvas, n: &SceneNode, alpha: f32) -> Result<(), RenderError> {
    let image = n.kind == "image";
    let b = Rect::from_bounds(&n.bounds);
    if image {
        if let Some(p) = geometry::path(b, [[0.0; 2]; 4]) {
            c.fill(&p, &color::solid("#ffe4e6", alpha)?, FillRule::Winding)?;
        }
    }
    let label = if image {
        "MISSING IMAGE"
    } else {
        "MISSING FONT"
    };
    let paint = color::solid(if image { "#9f1239" } else { "#be123c" }, alpha)?;
    for (i, ch) in label.chars().enumerate() {
        for (y, row) in bits(ch).iter().enumerate() {
            for x in 0..3 {
                if row & (1 << (2 - x)) != 0 {
                    let b = Rect {
                        x: b.x + 4.0 + i as f32 * 8.0 + x as f32 * 2.0,
                        y: b.y + 4.0 + y as f32 * 2.0,
                        w: 2.0,
                        h: 2.0,
                    };
                    if let Some(p) = geometry::path(b, [[0.0; 2]; 4]) {
                        c.fill(&p, &paint, FillRule::Winding)?;
                    }
                }
            }
        }
    }
    Ok(())
}
