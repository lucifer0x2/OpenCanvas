//! Bounded in-memory PNG/JPEG/WebP decode, EXIF orientation and ICC to sRGB.
use crate::{
    canvas::Canvas,
    geometry::{self, Rect},
    RenderError, RenderOptions,
};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use moxcms::DataColorSpace;
use opencanvas_layout::SceneNode;
use std::io::Cursor;
use tiny_skia::Pixmap;
#[path = "image_sample.rs"]
mod sample;

pub(crate) struct Decoded {
    pixmap: Pixmap,
    // Quantize bilinear coordinates in the original encoded image's axes.
    reversed: [bool; 2],
}
impl std::ops::Deref for Decoded {
    type Target = Pixmap;
    fn deref(&self) -> &Pixmap {
        &self.pixmap
    }
}

const MAX_BYTES: usize = 64 * 1024 * 1024;
fn error() -> RenderError {
    RenderError::new("RESOURCE_NOT_READY", "Image decode or color profile failed")
}
pub(crate) fn decode(
    bytes: &[u8],
    media: &str,
    options: &RenderOptions<'_>,
) -> Result<Decoded, RenderError> {
    if bytes.len() > MAX_BYTES {
        return Err(error());
    }
    let format = match media {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        "image/webp" => ImageFormat::WebP,
        _ => return Err(error()),
    };
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    limits.max_image_width = Some(16_000_000);
    limits.max_image_height = Some(16_000_000);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|_| error())?;
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > 16_000_000 {
        return Err(error());
    }
    let profile = crate::icc::profile(decoder.icc_profile().map_err(|_| error())?)?;
    let orientation = decoder
        .exif_metadata()
        .map_err(|_| error())?
        .and_then(|e| image::metadata::Orientation::from_exif_chunk(&e))
        .unwrap_or(decoder.orientation().map_err(|_| error())?);
    options.check()?;
    let mut image = if let Some(src) = profile
        .as_ref()
        .filter(|p| p.color_space == DataColorSpace::Cmyk)
    {
        if format != ImageFormat::Jpeg {
            return Err(RenderError::new(
                "UNSUPPORTED_FEATURE",
                "CMYK ICC requires CMYK JPEG source channels",
            ));
        }
        drop(decoder);
        DynamicImage::ImageRgba8(crate::icc::cmyk(bytes, src, w, h, options)?)
    } else {
        let mut image = DynamicImage::from_decoder(decoder)
            .map_err(|_| error())?
            .to_rgba8();
        if let Some(src) = &profile {
            crate::icc::apply(&mut image, src, options)?;
        }
        DynamicImage::ImageRgba8(image)
    };
    options.check()?;
    image.apply_orientation(orientation);
    let rgba = image.into_rgba8();
    let mut pixmap = Pixmap::new(rgba.width(), rgba.height()).ok_or_else(RenderError::budget)?;
    for (i, (src, dst)) in rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(pixmap.data_mut().as_chunks_mut::<4>().0.iter_mut())
        .enumerate()
    {
        if i % 4096 == 0 {
            options.check()?;
        }
        for k in 0..3 {
            dst[k] = ((u16::from(src[k]) * u16::from(src[3]) + 127) / 255) as u8;
        }
        dst[3] = src[3];
    }
    use image::metadata::Orientation::*;
    let reversed = match orientation {
        NoTransforms | Rotate90FlipH => [false, false],
        FlipHorizontal | Rotate90 => [true, false],
        FlipVertical | Rotate270 => [false, true],
        Rotate180 | Rotate270FlipH => [true, true],
    };
    Ok(Decoded { pixmap, reversed })
}
pub(crate) fn paint(
    c: &mut Canvas,
    n: &SceneNode,
    image: &Decoded,
    alpha: f32,
    opts: &RenderOptions<'_>,
) -> Result<(), RenderError> {
    let b = Rect::from_bounds(&n.bounds);
    let crop = n
        .crop
        .as_ref()
        .map(|v| [v.x, v.y, v.width, v.height])
        .unwrap_or([0.0, 0.0, 1.0, 1.0]);
    let sw = image.width() as f32 * crop[2];
    let sh = image.height() as f32 * crop[3];
    if sw <= 0.0 || sh <= 0.0 || b.w <= 0.0 || b.h <= 0.0 {
        return Ok(());
    }
    let (w, h) = if n.fit == "fit" || n.fit == "fill" {
        (b.w, b.h)
    } else {
        let r = if n.fit == "cover" {
            (b.w / sw).max(b.h / sh)
        } else {
            (b.w / sw).min(b.h / sh)
        };
        (sw * r, sh * r)
    };
    let x = b.x + (b.w - w) / 2.0;
    let y = b.y + (b.h - h) / 2.0;
    let old = c.clip.clone();
    if let Some(p) = geometry::node_path(n) {
        c.intersect_clip(&p)?;
    }
    if let Some(p) = geometry::path(Rect { x, y, w, h }, [[0.0; 2]; 4]) {
        let scale = c.scale;
        let sx = sw / (w * scale);
        let sy = sh / (h * scale);
        let tx = image.width() as f32 * crop[0] - x * scale * sx;
        let ty = image.height() as f32 * crop[1] - y * scale * sy;
        // Canvas drawImage does not antialias an axis-aligned destination rect.
        // Only the separately saved roundRect/ancestor clip contributes AA.
        c.shade(&p, Rect { x, y, w, h }, false, opts, |dx, dy| {
            sample::bilinear(
                image,
                (dx as f64 + 0.5) * sx as f64 + tx as f64 - 0.5,
                (dy as f64 + 0.5) * sy as f64 + ty as f64 - 0.5,
                alpha,
                image.reversed,
            )
        })?;
    }
    c.clip = old;
    Ok(())
}
