//! tiny-skia painter mirroring renderer/layers.ts + paint.ts + stroke.ts for the spike scene.
use crate::font::Shaper;
use crate::scene::{has_stroke, index, parse_color, Node, Scene, Tree};
use crate::text::{TextOpts, TextPainter};
use tiny_skia::*;

pub struct Painter<'a> {
    pub shaper: &'a Shaper,
    pub text: TextPainter,
    pub image: Pixmap,
    pub opts: TextOpts,
}

const K: f32 = 0.552_284_75;

/// Canvas roundRect with per-corner elliptical radii [tl, tr, br, bl] as (rx, ry).
pub fn round_rect(pb: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32, r: [(f32, f32); 4]) {
    let mut r = r;
    let f = [w / (r[0].0 + r[1].0), w / (r[3].0 + r[2].0), h / (r[0].1 + r[3].1), h / (r[1].1 + r[2].1)]
        .iter().fold(1f32, |m, v| if v.is_finite() { m.min(*v) } else { m });
    if f < 1.0 { for c in r.iter_mut() { c.0 *= f; c.1 *= f; } }
    let [tl, tr, br, bl] = r;
    pb.move_to(x + tl.0, y);
    pb.line_to(x + w - tr.0, y);
    pb.cubic_to(x + w - tr.0 * (1.0 - K), y, x + w, y + tr.1 * (1.0 - K), x + w, y + tr.1);
    pb.line_to(x + w, y + h - br.1);
    pb.cubic_to(x + w, y + h - br.1 * (1.0 - K), x + w - br.0 * (1.0 - K), y + h, x + w - br.0, y + h);
    pb.line_to(x + bl.0, y + h);
    pb.cubic_to(x + bl.0 * (1.0 - K), y + h, x, y + h - bl.1 * (1.0 - K), x, y + h - bl.1);
    pb.line_to(x, y + tl.1);
    pb.cubic_to(x, y + tl.1 * (1.0 - K), x + tl.0 * (1.0 - K), y, x + tl.0, y);
    pb.close();
}

fn node_path(n: &Node) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let b = n.bounds;
    round_rect(&mut pb, b.x, b.y, b.width, b.height, [(n.radius, n.radius); 4]);
    pb.finish()
}

fn solid(color: &str, alpha: f32) -> Option<Paint<'static>> {
    let c = parse_color(color)?;
    let mut p = Paint::default();
    p.set_color(Color::from_rgba(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, c[3] as f32 / 255.0 * alpha)?);
    p.anti_alias = true;
    Some(p)
}

impl Painter<'_> {
    pub fn render(&self, scene: &Scene, scale: f32) -> Pixmap {
        let w = (scene.provenance.viewport.width * scale).ceil() as u32;
        let h = (scene.provenance.viewport.height * scale).ceil() as u32;
        let mut px = Pixmap::new(w, h).unwrap();
        let tree = index(scene);
        for &r in &tree.roots { self.draw(&mut px, scene, &tree, r, 1.0, None, scale); }
        px
    }

    #[allow(clippy::too_many_arguments)]
    fn draw(&self, px: &mut Pixmap, s: &Scene, t: &Tree, i: usize, alpha: f32, clip: Option<&Mask>, k: f32) {
        let a = alpha * s.nodes[i].opacity;
        if a <= 0.0 { return; }
        if a < 1.0 && t.units[i] >= 2 {
            // Offscreen layer at alpha 1, composited once (whole-canvas layer, same pixel grid).
            let mut layer = Pixmap::new(px.width(), px.height()).unwrap();
            self.contents(&mut layer, s, t, i, 1.0, None, k);
            let paint = PixmapPaint { opacity: a, blend_mode: BlendMode::SourceOver, quality: FilterQuality::Nearest };
            px.draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), clip);
            return;
        }
        self.contents(px, s, t, i, a, clip, k);
    }

    #[allow(clippy::too_many_arguments)]
    fn contents(&self, px: &mut Pixmap, s: &Scene, t: &Tree, i: usize, a: f32, clip: Option<&Mask>, k: f32) {
        let n = &s.nodes[i];
        let tf = Transform::from_scale(k, k);
        if n.kind != "text" {
            if let (Some(p), Some(path)) = (solid(&n.fill, a), node_path(n)) { px.fill_path(&path, &p, FillRule::Winding, tf, clip); }
        }
        if n.kind == "text" { self.paint_text(px, n, a, clip, k); }
        if n.kind == "image" { self.paint_image(px, n, a, clip, k); }
        if let Some(children) = t.children.get(&i) {
            let inner = if n.clip { Some(self.clip_mask(px, n, clip, k)) } else { clip.cloned() };
            for &c in children { self.draw(px, s, t, c, a, inner.as_ref(), k); }
        }
        if has_stroke(n) { self.paint_stroke(px, n, a, clip, k); }
    }

    fn clip_mask(&self, px: &Pixmap, n: &Node, clip: Option<&Mask>, k: f32) -> Mask {
        let path = node_path(n).unwrap();
        let tf = Transform::from_scale(k, k);
        match clip {
            Some(m) => { let mut m = m.clone(); m.intersect_path(&path, FillRule::Winding, true, tf); m }
            None => { let mut m = Mask::new(px.width(), px.height()).unwrap(); m.fill_path(&path, FillRule::Winding, true, tf); m }
        }
    }

    fn paint_text(&self, px: &mut Pixmap, n: &Node, a: f32, clip: Option<&Mask>, k: f32) {
        let mut pb = PathBuilder::new();
        for l in &n.lines {
            self.text.add_line(self.shaper, &mut pb, &l.text, n.bounds.x + l.x, n.bounds.y + l.y, n.font_size, k, self.opts);
        }
        let Some(mut mask) = self.text.coverage(pb, px.width(), px.height(), self.opts) else { return };
        if let Some(c) = clip {
            for (m, c) in mask.data_mut().iter_mut().zip(c.data()) { *m = ((*m as u32 * *c as u32 + 127) / 255) as u8; }
        }
        let Some(paint) = solid(&n.fill, a) else { return };
        let r = Rect::from_xywh(0.0, 0.0, px.width() as f32, px.height() as f32).unwrap();
        px.fill_rect(r, &paint, Transform::identity(), Some(&mask));
    }

    fn paint_image(&self, px: &mut Pixmap, n: &Node, a: f32, clip: Option<&Mask>, k: f32) {
        let mask = self.clip_mask(px, n, clip, k);
        let b = n.bounds;
        let (iw, ih) = (self.image.width() as f32, self.image.height() as f32);
        let c = n.crop.clone().unwrap_or(crate::scene::Crop { x: 0.0, y: 0.0, width: 1.0, height: 1.0 });
        let (sx, sy, sw, sh) = (iw * c.x, ih * c.y, iw * c.width, ih * c.height);
        let (mut dw, mut dh) = (b.width, b.height);
        if n.fit != "fit" && n.fit != "fill" {
            let (rx, ry) = (b.width / sw, b.height / sh);
            let ratio = if n.fit == "cover" { rx.max(ry) } else { rx.min(ry) };
            dw = sw * ratio; dh = sh * ratio;
        }
        let (dx, dy) = (b.x + (b.width - dw) / 2.0, b.y + (b.height - dh) / 2.0);
        let pt = Transform::from_translate(dx, dy).pre_scale(dw / sw, dh / sh).pre_translate(-sx, -sy);
        let mut paint = Paint::default();
        paint.anti_alias = true;
        paint.shader = Pattern::new(self.image.as_ref(), SpreadMode::Pad, FilterQuality::Bilinear, a, pt);
        let dest = Rect::from_xywh(dx, dy, dw, dh).unwrap();
        px.fill_path(&PathBuilder::from_rect(dest), &paint, FillRule::Winding, Transform::from_scale(k, k), Some(&mask));
    }

    fn paint_stroke(&self, px: &mut Pixmap, n: &Node, a: f32, clip: Option<&Mask>, k: f32) {
        let s = n.stroke.as_ref().unwrap();
        let f = match s.align.as_str() { "center" => 0.5, "outside" => 1.0, _ => 0.0 };
        let [t, r, bo, l] = s.width;
        let b = n.bounds;
        let (ox, oy, ow, oh) = (b.x - l * f, b.y - t * f, b.width + (l + r) * f, b.height + (t + bo) * f);
        let kk = [(l, t), (r, t), (r, bo), (l, bo)];
        let o_r: Vec<(f32, f32)> = kk.iter().map(|&(h, v)| if n.radius > 0.0 { (n.radius + h * f, n.radius + v * f) } else { (0.0, 0.0) }).collect();
        let i_r: Vec<(f32, f32)> = o_r.iter().zip(kk).map(|(p, (h, v))| ((p.0 - h).max(0.0), (p.1 - v).max(0.0))).collect();
        let mut pb = PathBuilder::new();
        round_rect(&mut pb, ox, oy, ow, oh, [o_r[0], o_r[1], o_r[2], o_r[3]]);
        let (iw, ih) = (ow - l - r, oh - t - bo);
        if iw > 0.0 && ih > 0.0 { round_rect(&mut pb, ox + l, oy + t, iw, ih, [i_r[0], i_r[1], i_r[2], i_r[3]]); }
        // Spike scene uses one color per stroke; per-side wedges are not ported.
        if let (Some(p), Some(path)) = (solid(&s.color[0], a), pb.finish()) {
            px.fill_path(&path, &p, FillRule::EvenOdd, Transform::from_scale(k, k), clip);
        }
    }
}
