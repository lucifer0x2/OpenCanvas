//! Integrity checks for original SVG binaries, independent of raster capabilities.
//! Fonts stay live and images stay encoded: no glyph outlines or ICC conversion.
use crate::{validation, PaintScene, RenderError, RenderOptions, ResourceReadiness, Resources};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use opencanvas_text::Font;
use std::io::Cursor;

/// Validate immutable identities, font structure/faces and bounded image payloads.
/// A valid color font/profile need not be supported by our raster painter to embed it.
pub fn inspect_svg_resources(
    scene: &PaintScene,
    resources: &Resources<'_>,
    options: &RenderOptions<'_>,
) -> Result<Vec<ResourceReadiness>, RenderError> {
    validation::resource_identities(scene, resources, options)?;
    let mut observed = Vec::new();
    let mut failed = !resources.readiness_diagnostics.is_empty();
    for font in &scene.fonts {
        options.check()?;
        let ready = resources.fonts.get(&font.id).is_some_and(|bytes| {
            Font::from_bytes(bytes.bytes, &font.hash)
                .and_then(|f| f.with_face(&font.family, font.weight, &font.style))
                .is_ok()
        });
        failed |= !ready;
        observed.push(readiness(&font.id, "font", ready, "FONT_MISSING"));
    }
    for asset in &scene.assets {
        options.check()?;
        let ready = resources
            .assets
            .get(&asset.id)
            .is_some_and(|bytes| image_valid(bytes.bytes, &asset.media_type));
        failed |= !ready;
        observed.push(readiness(&asset.id, "asset", ready, "ASSET_DECODE_FAILED"));
    }
    for node in &scene.nodes {
        options.check()?;
        let wanted = match node.kind.as_str() {
            "text" => Some((node.font_id.as_deref(), "font")),
            "image" => Some((node.asset_id.as_deref(), "asset")),
            _ => None,
        };
        if let Some((id, kind)) = wanted {
            failed |= !observed
                .iter()
                .any(|r| Some(r.id.as_str()) == id && r.kind == kind && r.status == "ready");
        }
    }
    options.check()?;
    observed.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.id.cmp(&b.id)));
    if options.strict && failed {
        let mut error = RenderError::new("RESOURCE_NOT_READY", "SVG resources are not ready");
        error.readiness = observed;
        return Err(error);
    }
    Ok(observed)
}

fn readiness(id: &str, kind: &str, ready: bool, code: &str) -> ResourceReadiness {
    ResourceReadiness {
        id: id.into(),
        kind: kind.into(),
        status: if ready { "ready" } else { "failed" }.into(),
        code: (!ready).then(|| code.into()),
    }
}

fn image_valid(bytes: &[u8], media: &str) -> bool {
    let format = match media {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        "image/webp" => ImageFormat::WebP,
        _ => return false,
    };
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    limits.max_image_width = Some(16_000_000);
    limits.max_image_height = Some(16_000_000);
    reader.limits(limits);
    let Ok(decoder) = reader.into_decoder() else {
        return false;
    };
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 16_000_000 {
        return false;
    }
    // Decode the original codec to reject truncated/corrupt payloads. Color-profile
    // interpretation is the SVG consumer's responsibility, not a raster CMS gate.
    DynamicImage::from_decoder(decoder).is_ok()
}
