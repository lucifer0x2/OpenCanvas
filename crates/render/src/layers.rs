//! CSS group opacity: integer-device-offset layers, live-pixel budget and fold fallback.
use crate::{
    canvas::Canvas,
    geometry::{self, Rect},
    paint,
    resources::Prepared,
    stroke,
    tree::Tree,
    DrawStats, PaintScene, RenderError, RenderOptions,
};

pub(crate) fn paint(
    c: &mut Canvas,
    s: &PaintScene,
    t: &Tree,
    r: &Prepared,
    o: &RenderOptions<'_>,
    stats: &mut DrawStats,
) -> Result<(), RenderError> {
    let mut ctx = Painter {
        scene: s,
        tree: t,
        resources: r,
        opts: o,
        stats,
        live: 0,
    };
    let clip = c.bounds();
    for i in &t.roots {
        ctx.draw(c, *i, 1.0, clip)?;
    }
    Ok(())
}
struct Painter<'a, 'b> {
    scene: &'a PaintScene,
    tree: &'a Tree,
    resources: &'a Prepared,
    opts: &'a RenderOptions<'b>,
    stats: &'a mut DrawStats,
    live: u64,
}
impl Painter<'_, '_> {
    fn draw(
        &mut self,
        c: &mut Canvas,
        i: usize,
        alpha: f32,
        clip: Rect,
    ) -> Result<(), RenderError> {
        self.opts.check()?;
        let n = &self.scene.nodes[i];
        let a = alpha * n.opacity;
        if a <= 0.0 {
            return Ok(());
        }
        if a < 1.0 && self.tree.units[i] >= 2 {
            let Some(r) = self.tree.extents[i].and_then(|e| e.device(c.scale, clip)) else {
                return Ok(());
            };
            let w = r.w as u32;
            let h = r.h as u32;
            let pixels = u64::from(w) * u64::from(h);
            if self.live + pixels <= self.opts.max_layer_pixels.min(16_000_000) {
                let mut layer = Canvas::new(w, h, c.scale, r.x as i32, r.y as i32)?;
                layer.cancelled = c.cancelled;
                self.stats.opacity_layers += 1;
                self.live += pixels;
                self.contents(&mut layer, i, 1.0, r)?;
                self.live -= pixels;
                c.composite(&layer, a);
                return Ok(());
            }
            self.stats.opacity_layer_fallbacks += 1;
        }
        self.contents(c, i, a, clip)
    }
    fn contents(
        &mut self,
        c: &mut Canvas,
        i: usize,
        alpha: f32,
        clip: Rect,
    ) -> Result<(), RenderError> {
        let n = &self.scene.nodes[i];
        paint::node(c, n, self.resources, alpha, self.opts)?;
        if !self.tree.children[i].is_empty() {
            let old = c.clip.clone();
            let inner = if n.clip {
                if let Some(path) = geometry::node_path(n) {
                    c.intersect_clip(&path)?;
                }
                if n.bounds.width <= 0.0 || n.bounds.height <= 0.0 {
                    None
                } else {
                    Rect::from_bounds(&n.bounds).device(c.scale, clip)
                }
            } else {
                Some(clip)
            };
            if let Some(inner) = inner {
                for child in &self.tree.children[i] {
                    self.draw(c, *child, alpha, inner)?;
                }
            }
            c.clip = old;
        }
        self.opts.check()?;
        // Deliberately after descendants, outside this node's own clip, inside ancestors.
        stroke::paint(c, n, alpha)
    }
}
