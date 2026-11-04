use crate::{Raster, RenderError};

/// Encode premultiplied raster to a straight-alpha, sRGB-tagged PNG in memory.
pub fn encode_png(raster: &Raster) -> Result<Vec<u8>, RenderError> {
    let expected = u64::from(raster.width) * u64::from(raster.height) * 4;
    if expected > 64_000_000 || expected as usize != raster.rgba.len() {
        return Err(RenderError::budget());
    }
    let mut rgba = raster.rgba.clone();
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = u32::from(px[3]);
        for c in &mut px[..3] {
            *c = (u32::from(*c) * 255 + a / 2)
                .checked_div(a)
                .unwrap_or(0)
                .min(255) as u8;
        }
    }
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, raster.width, raster.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
        let mut writer = encoder.write_header().map_err(error)?;
        writer.write_image_data(&rgba).map_err(error)?;
    }
    Ok(output)
}
fn error(_: png::EncodingError) -> RenderError {
    RenderError::new("ENCODE_FAILED", "Could not encode PNG")
}
