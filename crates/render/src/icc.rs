//! Color management operates on unassociated source channels, before orientation/premultiply.
use crate::{RenderError, RenderOptions};
use image::RgbaImage;
use moxcms::{ColorProfile, DataColorSpace, Layout, TransformOptions};
use std::io::Cursor;

fn error() -> RenderError {
    RenderError::new("RESOURCE_NOT_READY", "Image color profile failed")
}
pub(crate) fn profile(bytes: Option<Vec<u8>>) -> Result<Option<ColorProfile>, RenderError> {
    bytes
        .map(|bytes| {
            if bytes.len() > 4 * 1024 * 1024 {
                return Err(error());
            }
            ColorProfile::new_from_slice(&bytes).map_err(|_| error())
        })
        .transpose()
}
pub(crate) fn apply(
    image: &mut RgbaImage,
    src: &ColorProfile,
    opts: &RenderOptions<'_>,
) -> Result<(), RenderError> {
    let (layout, input) = match src.color_space {
        DataColorSpace::Rgb => (Layout::Rgba, image.as_raw().clone()),
        DataColorSpace::Gray => (
            Layout::GrayAlpha,
            image.pixels().flat_map(|p| [p[0], p[3]]).collect(),
        ),
        _ => {
            return Err(RenderError::new(
                "UNSUPPORTED_FEATURE",
                "ICC color space does not match decoded RGB/gray channels",
            ))
        }
    };
    let stride = if layout == Layout::Rgba { 4 } else { 2 };
    transform(src, layout, &input, image, stride, opts)
}
fn transform(
    src: &ColorProfile,
    layout: Layout,
    input: &[u8],
    out: &mut RgbaImage,
    stride: usize,
    opts: &RenderOptions<'_>,
) -> Result<(), RenderError> {
    let transform = src
        .create_transform_8bit(
            layout,
            &ColorProfile::new_srgb(),
            Layout::Rgba,
            TransformOptions::default(),
        )
        .map_err(|_| error())?;
    let width = out.width() as usize;
    for (row, dst) in input
        .chunks_exact(width * stride)
        .zip(out.as_mut().chunks_exact_mut(width * 4))
    {
        opts.check()?;
        transform.transform(row, dst).map_err(|_| error())?;
    }
    Ok(())
}
pub(crate) fn cmyk(
    bytes: &[u8],
    src: &ColorProfile,
    w: u32,
    h: u32,
    opts: &RenderOptions<'_>,
) -> Result<RgbaImage, RenderError> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    decoder.set_max_decoding_buffer_size(128 * 1024 * 1024);
    decoder.read_info().map_err(|_| error())?;
    let info = decoder.info().ok_or_else(error)?;
    if info.pixel_format != jpeg_decoder::PixelFormat::CMYK32
        || u32::from(info.width) != w
        || u32::from(info.height) != h
    {
        return Err(error());
    }
    opts.check()?;
    // jpeg-decoder returns conventional (not Adobe-inverted) C,M,Y,K for both CMYK/YCCK.
    let input = decoder.decode().map_err(|_| error())?;
    opts.check()?;
    let mut out = RgbaImage::new(w, h);
    transform(src, Layout::Rgba, &input, &mut out, 4, opts)?;
    // Four source components are inks, NOT RGBA alpha; destination is fully opaque.
    for row in out.as_mut().chunks_exact_mut(w as usize * 4) {
        opts.check()?;
        for p in row.as_chunks_mut::<4>().0 {
            p[3] = 255;
        }
    }
    Ok(out)
}
