//! tiny-skia painter mirroring renderer/layers.ts + paint.ts + stroke.ts for the spike scenes.
//! Text goes through text_mask.rs (bbox, default) or text_full.rs (the original, `--paint full`);
//! clip masks through clip.rs. Everything else is shared by both modes.
use crate::clip;
use crate::font::Shaper;
use crate::mode::PaintMode;
use crate::profile::PaintProfile;
use crate::scene::{has_stroke, index, Node, Scene, Tree};
use crate::shape::{node_path, round_rect, solid};
use crate::text::{TextOpts, TextPainter};
use crate::text_mask::{self, TextMask};
use crate::{par, text_full};
use std::sync::Mutex;
use std::time::Instant;
use tiny_skia::*;

pub struct Painter<'a> {
    pub shaper: &'a Shaper,
    pub text: TextPainter,
    pub image: Pixmap,
    pub opts: TextOpts,
    pub mode: PaintMode,
    /// Profile of the last `render`.
    pub prof: Mutex<PaintProfile>,
}

/// One render: the scene and its paint tree (shared), pre-rasterized text, and a zeroed
/// canvas-size mask that text_mask::composite borrows (created on first use).
struct Job<'s> {
    s: &'s Scene,
    t: &'s Tree,
    k: f32,
    texts: Vec<Option<TextMask>>,
    scratch: Option<Mask>,
    prof: PaintProfile,
}

fn ms(t: Instant) -> f64 { t.elapsed().as_secs_f64() * 1e3 }

impl Painter<'_> {
    pub fn render(&self, scene: &Scene, scale: f32) -> Pixmap {
        let w = (scene.provenance.viewport.width * scale).ceil() as u32;
        let h = (scene.provenance.viewport.height * scale).ceil() as u32;
        let mut px = Pixmap::new(w, h).unwrap();
        let tree = index(scene);
        let mut prof = PaintProfile { threads: par::threads() as u32, ..Default::default() };
        let texts = match self.mode {
            PaintMode::Bbox => text_mask::raster_all(&self.text, self.shaper, self.opts, scene, &tree, scale, (w, h), &mut prof),
            PaintMode::Full => Vec::new(),
        };
        let mut job = Job { s: scene, t: &tree, k: scale, texts, scratch: None, prof };
        for &r in &tree.roots { self.draw(&mut job, &mut px, r, 1.0, None); }
        *self.prof.lock().unwrap() = job.prof;
        px
    }

    fn draw(&self, job: &mut Job, px: &mut Pixmap, i: usize, alpha: f32, clip: Option<&Mask>) {
        let a = alpha * job.s.nodes[i].opacity;
        if a <= 0.0 { return; }
        if a < 1.0 && job.t.units[i] >= 2 {
            // Offscreen layer at alpha 1, composited once (whole-canvas layer, same pixel grid).
            let mut layer = Pixmap::new(px.width(), px.height()).unwrap();
            self.contents(job, &mut layer, i, 1.0, None);
            let paint = PixmapPaint { opacity: a, blend_mode: BlendMode::SourceOver, quality: FilterQuality::Nearest };
            px.draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), clip);
            return;
        }
        self.contents(job, px, i, a, clip);
    }

    fn contents(&self, job: &mut Job, px: &mut Pixmap, i: usize, a: f32, clip: Option<&Mask>) {
        let (s, t, k) = (job.s, job.t, job.k);
        let n = &s.nodes[i];
        let tf = Transform::from_scale(k, k);
        let started = Instant::now();
        if n.kind != "text" {
            if let (Some(p), Some(path)) = (solid(&n.fill, a), node_path(n)) { px.fill_path(&path, &p, FillRule::Winding, tf, clip); }
        }
        if n.kind == "image" { self.paint_image(px, n, a, clip, k); }
        job.prof.shape_ms += ms(started);
        if n.kind == "text" { self.paint_text(job, px, i, a, clip); }
        if let Some(children) = t.children.get(&i) {
            let started = Instant::now();
            // Only clipping nodes need a mask of their own; the full painter also copied the
            // inherited one for every other parent.
            let own = match (n.clip, self.mode) {
                (true, mode) => Some(clip::mask(px.width(), px.height(), n, clip, k, mode)),
                (false, PaintMode::Full) => clip.cloned(),
                (false, PaintMode::Bbox) => None,
            };
            job.prof.masks += own.is_some() as u32;
            job.prof.mask_ms += ms(started);
            let inner = if own.is_some() { own.as_ref() } else { clip };
            for &c in children { self.draw(job, px, c, a, inner); }
        }
        if has_stroke(n) {
            let started = Instant::now();
            self.paint_stroke(px, n, a, clip, k);
            job.prof.shape_ms += ms(started);
        }
    }

    fn paint_text(&self, job: &mut Job, px: &mut Pixmap, i: usize, a: f32, clip: Option<&Mask>) {
        let n = &job.s.nodes[i];
        if self.mode == PaintMode::Full {
            text_full::paint_text(&self.text, self.shaper, self.opts, px, n, a, clip, job.k, &mut job.prof);
            return;
        }
        let t = Instant::now();
        let (Some(m), Some(paint)) = (job.texts[i].take(), solid(&n.fill, a)) else { return };
        let scratch = job.scratch.get_or_insert_with(|| Mask::new(px.width(), px.height()).unwrap());
        text_mask::composite(px, scratch, &m, &paint, clip);
        job.prof.composite_ms += ms(t);
    }

    fn paint_image(&self, px: &mut Pixmap, n: &Node, a: f32, clip: Option<&Mask>, k: f32) {
        let mask = clip::mask(px.width(), px.height(), n, clip, k, self.mode);
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
        let paint = Paint {
            anti_alias: true,
            shader: Pattern::new(self.image.as_ref(), SpreadMode::Pad, FilterQuality::Bilinear, a, pt),
            ..Default::default()
        };
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
