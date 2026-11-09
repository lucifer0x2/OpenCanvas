//! Reject malformed untrusted scenes before any destination or layer allocation.
use crate::{color, resources, PaintScene, RenderError, RenderOptions, Resources};
use opencanvas_layout::{Gradient, Paint, Radius, SceneNode, RENDERER_VERSION, SCENE_VERSION};
use std::collections::BTreeSet;

pub(crate) fn resource_identities(
    s: &PaintScene,
    r: &Resources<'_>,
    o: &RenderOptions<'_>,
) -> Result<(), RenderError> {
    if s.provenance.scene_version != SCENE_VERSION
        || s.provenance.renderer_version != RENDERER_VERSION
    {
        return Err(RenderError::new(
            "PIPELINE_MISMATCH",
            "Scene pipeline version mismatch",
        ));
    }
    if s.fonts.len() > 2048 || s.assets.len() > 2048 {
        return Err(RenderError::scene("Scene resource budget invalid"));
    }
    let mut ids = BTreeSet::new();
    for f in &s.fonts {
        o.check()?;
        if !ids.insert(&f.id) {
            return Err(RenderError::scene("Duplicate font manifest id"));
        }
        if let Some(b) = r.fonts.get(&f.id) {
            if b.bytes.len() > 64 * 1024 * 1024 {
                return Err(RenderError::new(
                    "RESOURCE_LIMIT",
                    "Font byte limit exceeded",
                ));
            }
            resources::verify(*b, &f.hash)?;
        }
    }
    let mut ids = BTreeSet::new();
    for a in &s.assets {
        o.check()?;
        if !ids.insert(&a.id) {
            return Err(RenderError::scene("Duplicate asset manifest id"));
        }
        if let Some(b) = r.assets.get(&a.id) {
            if b.bytes.len() > 64 * 1024 * 1024 {
                return Err(RenderError::new(
                    "RESOURCE_LIMIT",
                    "Image byte limit exceeded",
                ));
            }
            resources::verify(*b, &a.hash)?;
        }
    }
    Ok(())
}

pub(crate) fn validate(
    s: &PaintScene,
    r: &Resources<'_>,
    o: &RenderOptions<'_>,
) -> Result<(u32, u32), RenderError> {
    resource_identities(s, r, o)?;
    let v = &s.provenance.viewport;
    let w = (v.width * o.scale).ceil();
    let h = (v.height * o.scale).ceil();
    if !o.scale.is_finite()
        || o.scale <= 0.0
        || o.scale > 4.0
        || !w.is_finite()
        || !h.is_finite()
        || w < 1.0
        || h < 1.0
        || w > 16_000_000.0
        || h > 16_000_000.0
        || w * h > 16_000_000.0
    {
        return Err(RenderError::budget());
    }
    if !o.text_coverage_gamma.is_finite() || !(0.25..=4.0).contains(&o.text_coverage_gamma) {
        return Err(RenderError::new(
            "VALIDATION_FAILED",
            "Invalid text coverage gamma",
        ));
    }
    if s.schema_version != 1
        || s.coordinate_space != "frame-local"
        || s.nodes.len() > 2048
        || s.fonts.len() > 2048
        || s.assets.len() > 2048
    {
        return Err(RenderError::scene(
            "Scene schema, coordinate space or node budget invalid",
        ));
    }
    let mut text_bytes = 0usize;
    for n in &s.nodes {
        o.check()?;
        node(n)?;
        for l in &n.lines {
            text_bytes = text_bytes.saturating_add(l.text.len());
        }
    }
    if text_bytes > 3 * 1024 * 1024 {
        return Err(RenderError::scene("Scene text budget exceeded"));
    }
    Ok((w as u32, h as u32))
}
fn finite(v: f32) -> bool {
    // A bounded 2048-node hug tree / long line can legitimately exceed the viewport
    // by millions of units. Keep it renderable; device allocation is clipped separately.
    v.is_finite() && v.abs() <= 1_000_000_000.0
}
fn positive(v: f32) -> bool {
    finite(v) && v >= 0.0
}
fn node(n: &SceneNode) -> Result<(), RenderError> {
    let invalid = || RenderError::scene("Invalid computed node paint geometry");
    let b = &n.bounds;
    if ![b.x, b.y].into_iter().all(finite)
        || ![b.width, b.height].into_iter().all(positive)
        || !n.opacity.is_finite()
        || !(0.0..=1.0).contains(&n.opacity)
        || !n.radius.valid_shape()
        || n.radius.each().iter().any(|r| !positive(*r))
        || !["page", "frame", "group", "rectangle", "image", "text"].contains(&n.kind.as_str())
    {
        return Err(invalid());
    }
    if !n.font_size.is_finite()
        || !(1.0..=10000.0).contains(&n.font_size)
        || !positive(n.line_height)
        || !n.letter_spacing.is_finite()
        || !(-1000.0..=1000.0).contains(&n.letter_spacing)
    {
        return Err(invalid());
    }
    for l in &n.lines {
        if ![l.x, l.y].into_iter().all(finite)
            || !positive(l.width)
            || l.text.len() > opencanvas_text::MAX_RUN_BYTES
        {
            return Err(invalid());
        }
        if let Some(b) = &l.ink_bounds {
            if ![b.x, b.y].into_iter().all(finite) || ![b.width, b.height].into_iter().all(positive)
            {
                return Err(invalid());
            }
        }
    }
    if let Some(d) = &n.decoration {
        if !finite(d.offset) || !positive(d.thickness) {
            return Err(invalid());
        }
    }
    if let Some(c) = &n.crop {
        if ![c.x, c.y, c.width, c.height]
            .into_iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(&v))
            || c.width <= 0.0
            || c.height <= 0.0
            || c.x + c.width > 1.0
            || c.y + c.height > 1.0
        {
            return Err(invalid());
        }
    }
    if !["fit", "fill", "contain", "cover"].contains(&n.fit.as_str()) {
        return Err(invalid());
    }
    if let Some(s) = &n.stroke {
        if !s.width.into_iter().all(positive)
            || !["inside", "center", "outside"].contains(&s.align.as_str())
        {
            return Err(invalid());
        }
        for c in &s.color {
            color::color(c)?;
        }
    }
    if n.shadow.len() > 8 {
        return Err(invalid());
    }
    for s in &n.shadow {
        if ![s.x, s.y].into_iter().all(finite)
            || !s.blur.is_finite()
            || !(0.0..=1000.0).contains(&s.blur)
            || !s.spread.is_finite()
            || !(-1000.0..=1000.0).contains(&s.spread)
        {
            return Err(invalid());
        }
        color::color(&s.color)?;
    }
    match &n.fill {
        Paint::Solid(s) => {
            color::color(s)?;
        }
        Paint::Gradient(g) => {
            if n.kind == "text" {
                return Err(RenderError::new(
                    "UNSUPPORTED_FEATURE",
                    "Text gradient is not a scene-3 paint",
                ));
            }
            let stops = match g {
                Gradient::Linear { angle, stops } => {
                    if !angle.is_finite() {
                        return Err(invalid());
                    }
                    stops
                }
                Gradient::Radial {
                    center,
                    radius,
                    stops,
                } => {
                    if !center.iter().all(|v| finite(*v)) {
                        return Err(invalid());
                    }
                    let r = match radius {
                        None => [1.0; 2],
                        Some(Radius::Circle(r)) => [*r; 2],
                        Some(Radius::Ellipse(r)) => *r,
                    };
                    if !r.into_iter().all(|r| positive(r) && r > 0.0) {
                        return Err(invalid());
                    }
                    stops
                }
            };
            if !(2..=16).contains(&stops.len())
                || stops
                    .iter()
                    .any(|(p, _)| !p.is_finite() || !(0.0..=1.0).contains(p))
                || stops.windows(2).any(|p| p[0].0 > p[1].0)
            {
                return Err(invalid());
            }
            for (_, c) in stops {
                color::color(c)?;
            }
        }
    }
    Ok(())
}
