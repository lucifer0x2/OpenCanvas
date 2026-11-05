use crate::{images, PaintScene, RenderError, RenderOptions, ResourceReadiness};
use opencanvas_text::{Font, FontSet};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Hash is the original file's lowercase SHA256, including for WOFF2 fonts.
#[derive(Clone, Copy)]
pub struct ResourceBytes<'a> {
    pub hash: &'a str,
    pub bytes: &'a [u8],
}
/// Immutable caller-owned bytes, keyed by manifest ID. URLs are never consulted.
#[derive(Default)]
pub struct Resources<'a> {
    pub fonts: BTreeMap<String, ResourceBytes<'a>>,
    pub assets: BTreeMap<String, ResourceBytes<'a>>,
    /// Upstream load failures, not scene advisories. Nonempty fails strict rendering.
    pub readiness_diagnostics: Vec<Value>,
}
pub(crate) struct Prepared {
    pub fonts: FontSet,
    pub images: BTreeMap<String, images::Decoded>,
    pub diagnostics: Vec<Value>,
    pub readiness: Vec<ResourceReadiness>,
}
pub(crate) fn verify(bytes: ResourceBytes<'_>, hash: &str) -> Result<(), RenderError> {
    if bytes.hash != hash || format!("{:x}", Sha256::digest(bytes.bytes)) != hash {
        return Err(RenderError::new(
            "PIPELINE_MISMATCH",
            "Resource manifest mismatch",
        ));
    }
    Ok(())
}
impl Resources<'_> {
    pub(crate) fn prepare(
        &self,
        scene: &PaintScene,
        options: &RenderOptions<'_>,
    ) -> Result<Prepared, RenderError> {
        let mut p = Prepared {
            fonts: FontSet::new(),
            images: BTreeMap::new(),
            diagnostics: scene
                .diagnostics
                .iter()
                .chain(&self.readiness_diagnostics)
                .cloned()
                .collect(),
            readiness: Vec::new(),
        };
        let mut not_ready = !self.readiness_diagnostics.is_empty();
        for f in &scene.fonts {
            options.check()?;
            if let Some(bytes) = self.fonts.get(&f.id) {
                match Font::from_bytes(bytes.bytes, &f.hash)
                    .and_then(|font| font.with_face(&f.family, f.weight, &f.style))
                {
                    Ok(font) => {
                        reject_color_font(&font).map_err(|mut error| {
                            error.readiness = p.readiness.clone();
                            error
                                .readiness
                                .push(readiness(&f.id, "font", false, &error.code));
                            error
                        })?;
                        p.fonts
                            .insert(&f.id, font)
                            .map_err(|_| RenderError::scene("Invalid font identity"))?;
                    }
                    Err(_) => {
                        not_ready = true;
                        p.diagnostics
                            .push(json!({"code":"FONT_MISSING","fontId":f.id}));
                    }
                }
            } else {
                not_ready = true;
                p.diagnostics
                    .push(json!({"code":"FONT_MISSING","fontId":f.id}));
            }
            p.readiness.push(readiness(
                &f.id,
                "font",
                p.fonts.get(&f.id).is_some(),
                "FONT_MISSING",
            ));
        }
        for a in &scene.assets {
            options.check()?;
            if let Some(bytes) = self.assets.get(&a.id) {
                match images::decode(bytes.bytes, &a.media_type, options) {
                    Ok(image) => {
                        p.images.insert(a.id.clone(), image);
                    }
                    Err(e) if e.code == "CANCELLED" => return Err(e),
                    Err(mut e) if e.code == "UNSUPPORTED_FEATURE" => {
                        e.readiness = p.readiness.clone();
                        e.readiness.push(readiness(&a.id, "asset", false, &e.code));
                        return Err(e);
                    }
                    Err(_) => {
                        not_ready = true;
                        p.diagnostics
                            .push(json!({"code":"ASSET_MISSING","assetId":a.id}));
                    }
                }
            } else {
                not_ready = true;
                p.diagnostics
                    .push(json!({"code":"ASSET_MISSING","assetId":a.id}));
            }
            p.readiness.push(readiness(
                &a.id,
                "asset",
                p.images.contains_key(&a.id),
                "ASSET_DECODE_FAILED",
            ));
        }
        for n in &scene.nodes {
            options.check()?;
            let font = n.font_id.as_ref().and_then(|id| p.fonts.get(id));
            let missing = (n.kind == "text" && font.is_none())
                || (n.kind == "image"
                    && !n
                        .asset_id
                        .as_ref()
                        .is_some_and(|id| p.images.contains_key(id)));
            if missing {
                not_ready = true;
                p.diagnostics
                    .push(json!({"code":"RESOURCE_NOT_READY","nodeId":n.id}));
                if !options.strict {
                    p.diagnostics
                        .push(json!({"code":"PLACEHOLDER_BITMAP_LABEL","nodeId":n.id}));
                }
            }
            if n.kind == "text" {
                if let Some(font) = font {
                    for l in &n.lines {
                        let missing = font
                            .missing_codepoints(&l.text)
                            .map_err(|_| RenderError::scene("Invalid text run"))?;
                        if !missing.is_empty() {
                            p.diagnostics.push(json!({"code":"MISSING_GLYPH","nodeId":n.id,"fontId":n.font_id,"codepoints":missing}));
                        }
                    }
                }
            }
        }
        options.check()?;
        p.readiness
            .sort_by(|a, b| a.kind.cmp(&b.kind).then(a.id.cmp(&b.id)));
        if options.strict && not_ready {
            let mut error = RenderError::new("RESOURCE_NOT_READY", "Scene resources are not ready");
            error.readiness = p.readiness;
            return Err(error);
        }
        Ok(p)
    }
}
fn readiness(id: &str, kind: &str, ready: bool, code: &str) -> ResourceReadiness {
    ResourceReadiness {
        id: id.into(),
        kind: kind.into(),
        status: if ready { "ready" } else { "failed" }.into(),
        code: (!ready).then(|| code.into()),
    }
}
fn reject_color_font(font: &Font) -> Result<(), RenderError> {
    let sfnt = font.sfnt_bytes();
    // The text foundation has already validated the SFNT directory. Color tables
    // cannot be painted as monochrome outlines without silently changing the design.
    let count = u16::from_be_bytes([sfnt[4], sfnt[5]]) as usize;
    if sfnt[12..12 + count * 16]
        .as_chunks::<16>()
        .0
        .iter()
        .any(|entry| matches!(&entry[..4], b"COLR" | b"CBDT" | b"sbix" | b"SVG "))
    {
        return Err(RenderError::new(
            "UNSUPPORTED_FEATURE",
            "Color-font tables require a color glyph painter",
        ));
    }
    Ok(())
}
