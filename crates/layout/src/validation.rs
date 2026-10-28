use crate::model::*;
use crate::util::{err, finite, is_fill};
use crate::{MAX_NODES, MAX_PIXELS};
use std::collections::{HashMap, HashSet};
mod layout;

pub(crate) fn validate(input: &LayoutInput) -> Result<(), LayoutError> {
    if input.schema_version != 1
        || !finite(input.viewport.width)
        || !finite(input.viewport.height)
        || input.viewport.width < 1.0
        || input.viewport.height < 1.0
        || input.viewport.width * input.viewport.height > MAX_PIXELS
        || input.nodes.len() > MAX_NODES
    {
        return Err(err("Schema, node or viewport budget exceeded"));
    }
    let mut ids = HashMap::new();
    let mut text_bytes = 0;
    for n in &input.nodes {
        if ids.insert(&n.id, n).is_some() {
            return Err(err("Duplicate node ID"));
        }
        if !["page", "frame", "group", "rectangle", "text", "image"].contains(&n.kind.as_str()) {
            return Err(err("Unsupported node kind"));
        }
        let p = &n.props;
        layout::properties(n)?;
        for d in [&p.width, &p.height] {
            match d {
                Some(Sizing::Fixed(v)) if !finite(*v) => return Err(err("Invalid dimension")),
                Some(Sizing::Rule(r)) if !["fill", "hug"].contains(&r.as_str()) => {
                    return Err(err("Unsupported sizing rule"));
                }
                _ => {}
            }
        }
        for v in [
            p.font_size,
            p.line_height,
            p.min_width,
            p.max_width,
            p.min_height,
            p.max_height,
            p.radius,
        ] {
            if v.is_some_and(|v| !finite(v)) {
                return Err(err("Invalid numeric property"));
            }
        }
        if p.font_size.is_some_and(|v| v < 1.0)
            || p.line_height.is_some_and(|v| v < 1.0)
            || p.opacity
                .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
            || [p.x, p.y]
                .into_iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 100_000.0)
        {
            return Err(err("Invalid font, position or opacity"));
        }
        if p.text.as_ref().is_some_and(|t| t.len() > 64 * 1024) {
            return Err(err("Text exceeds 64 KiB"));
        }
        text_bytes += p.text.as_ref().map_or(0, String::len);
        if text_bytes > 1024 * 1024 {
            return Err(err("Frame text exceeds 1 MiB budget"));
        }
        if crate::scene_fit(p.fit.as_deref()).is_none() {
            return Err(err("Unsupported image fit"));
        }
        if let Some(c) = &p.crop
            && (![c.x, c.y, c.width, c.height].into_iter().all(finite)
                || c.width <= 0.0
                || c.height <= 0.0
                || c.x + c.width > 1.0
                || c.y + c.height > 1.0)
        {
            return Err(err("Crop must be a normalized source rectangle"));
        }
        if let Some(l) = &p.layout
            && (!["row", "column", "free"].contains(&l.mode.as_str())
                || !finite(l.padding)
                || !finite(l.gap))
        {
            return Err(err("Unsupported layout"));
        }
    }
    if ids
        .get(&input.frame_id)
        .is_none_or(|node| node.kind != "frame")
    {
        return Err(err("Frame scope must reference a frame"));
    }
    for n in &input.nodes {
        let mut seen = HashSet::new();
        let mut current = n;
        while let Some(parent) = &current.parent_id {
            if !seen.insert(parent) {
                return Err(err("Tree cycle"));
            }
            if seen.len() > 128 {
                return Err(err("Tree depth exceeds 128"));
            }
            current = ids
                .get(parent)
                .copied()
                .ok_or_else(|| err("Missing parent"))?;
        }
        layout::relationships(n, input, &ids)?;
        if let Some(parent) = n.parent_id.as_ref().and_then(|p| ids.get(p)) {
            if !["page", "frame", "group"].contains(&parent.kind.as_str()) {
                return Err(err("Leaf cannot parent nodes"));
            }
            if parent.id != input.frame_id {
                for (child, parent) in [
                    (&n.props.width, &parent.props.width),
                    (&n.props.height, &parent.props.height),
                ] {
                    if is_fill(child)
                        && (parent.is_none() || matches!(parent,Some(Sizing::Rule(r)) if r=="hug"))
                    {
                        return Err(err("Fill inside hug axis is ambiguous"));
                    }
                }
            }
        }
    }
    Ok(())
}
