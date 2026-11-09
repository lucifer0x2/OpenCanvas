//! A drawing target with a device-integer origin, inherited AA clip, and scale.
use crate::{geometry::Rect, RenderError, RenderOptions};
use tiny_skia::{
    FillRule, FilterQuality, Mask, Paint, Path, Pattern, Pixmap, PixmapPaint, SpreadMode, Transform,
};
#[path = "coverage.rs"]
mod coverage;

pub(crate) struct Canvas<'a> {
    pub pixmap: Pixmap,
    pub scale: f32,
    pub x: i32,
    pub y: i32,
    pub clip: Option<Mask>,
    pub cancelled: Option<&'a dyn Fn() -> bool>,
}
impl Canvas<'_> {
    pub fn new(w: u32, h: u32, scale: f32, x: i32, y: i32) -> Result<Self, RenderError> {
        Ok(Self {
            pixmap: Pixmap::new(w, h).ok_or_else(RenderError::budget)?,
            scale,
            x,
            y,
            clip: None,
            cancelled: None,
        })
    }
    pub fn transform(&self) -> Transform {
        Transform::from_row(
            self.scale,
            0.0,
            0.0,
            self.scale,
            -(self.x as f32),
            -(self.y as f32),
        )
    }
    pub fn bounds(&self) -> Rect {
        Rect {
            x: self.x as f32,
            y: self.y as f32,
            w: self.pixmap.width() as f32,
            h: self.pixmap.height() as f32,
        }
    }
    pub fn fill(
        &mut self,
        path: &Path,
        paint: &Paint<'_>,
        rule: FillRule,
    ) -> Result<(), RenderError> {
        self.fill_coverage(path, paint, rule, true)
    }
    fn fill_coverage(
        &mut self,
        path: &Path,
        paint: &Paint<'_>,
        rule: FillRule,
        aa: bool,
    ) -> Result<(), RenderError> {
        let transform = self.transform();
        self.check()?;
        let mut mask = if aa {
            self.path_mask(path, rule)?
        } else {
            let mut mask = Mask::new(self.pixmap.width(), self.pixmap.height())
                .ok_or_else(RenderError::budget)?;
            mask.fill_path(path, rule, false, transform);
            mask
        };
        if let Some(clip) = &self.clip {
            for (i, (a, b)) in mask.data_mut().iter_mut().zip(clip.data()).enumerate() {
                if i % 4096 == 0 {
                    self.check()?;
                }
                *a = ((u16::from(*a) * u16::from(*b) + 127) / 255) as u8;
            }
        }
        let b = self.bounds();
        let rect = tiny_skia::Rect::from_xywh(
            b.x / self.scale - 1.0,
            b.y / self.scale - 1.0,
            b.w / self.scale + 2.0,
            b.h / self.scale + 2.0,
        )
        .ok_or_else(|| RenderError::scene("Invalid canvas bounds"))?;
        self.pixmap.fill_rect(rect, paint, transform, Some(&mask));
        self.check()
    }
    /// Evaluate a device-pixel shader before the normal AA path/ancestor clipping.
    /// The intermediate never exceeds the target's bounded pixel area. Sampling
    /// is nearest here: the supplied shader has already filtered/quantized once.
    pub fn shade(
        &mut self,
        path: &Path,
        bounds: Rect,
        antialias: bool,
        opts: &RenderOptions<'_>,
        mut pixel: impl FnMut(f32, f32) -> [u8; 4],
    ) -> Result<(), RenderError> {
        let Some(b) = bounds.device(self.scale, self.bounds()) else {
            return Ok(());
        };
        let (w, h) = (b.w as u32, b.h as u32);
        if u64::from(w) * u64::from(h) > 16_000_000 {
            return Err(RenderError::budget());
        }
        let mut sampled = Pixmap::new(w, h).ok_or_else(RenderError::budget)?;
        for (i, dst) in sampled
            .data_mut()
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .enumerate()
        {
            if i % 4096 == 0 {
                opts.check()?;
            }
            *dst = pixel(b.x + (i % w as usize) as f32, b.y + (i / w as usize) as f32);
        }
        let t = Transform::from_row(
            1.0 / self.scale,
            0.0,
            0.0,
            1.0 / self.scale,
            b.x / self.scale,
            b.y / self.scale,
        );
        self.fill_coverage(
            path,
            &Paint {
                shader: Pattern::new(
                    sampled.as_ref(),
                    SpreadMode::Pad,
                    FilterQuality::Nearest,
                    1.0,
                    t,
                ),
                anti_alias: true,
                ..Paint::default()
            },
            FillRule::Winding,
            antialias,
        )?;
        Ok(())
    }
    pub fn path_mask(&self, path: &Path, rule: FillRule) -> Result<Mask, RenderError> {
        coverage::raster(
            path,
            rule,
            self.transform(),
            self.pixmap.width(),
            self.pixmap.height(),
            self.cancelled,
        )
    }
    pub fn intersect_clip(&mut self, path: &Path) -> Result<(), RenderError> {
        let mut mask = self.path_mask(path, FillRule::Winding)?;
        if let Some(old) = &self.clip {
            for (i, (a, b)) in mask.data_mut().iter_mut().zip(old.data()).enumerate() {
                if i % 4096 == 0 {
                    self.check()?;
                }
                *a = ((u16::from(*a) * u16::from(*b) + 127) / 255) as u8;
            }
        }
        self.clip = Some(mask);
        Ok(())
    }
    fn check(&self) -> Result<(), RenderError> {
        if self.cancelled.is_some_and(|f| f()) {
            Err(RenderError::new("CANCELLED", "Render cancelled by caller"))
        } else {
            Ok(())
        }
    }
    pub fn composite(&mut self, layer: &Canvas, alpha: f32) {
        self.pixmap.draw_pixmap(
            layer.x - self.x,
            layer.y - self.y,
            layer.pixmap.as_ref(),
            &PixmapPaint {
                opacity: alpha,
                ..PixmapPaint::default()
            },
            Transform::identity(),
            self.clip.as_ref(),
        );
    }
}
