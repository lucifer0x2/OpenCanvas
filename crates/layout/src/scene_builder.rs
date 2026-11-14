//! Build the existing scene contract from computed geometry and final-width lines.
use crate::measurement::MeasurementSource;
use crate::model::*;
use crate::util::*;
use crate::{MAX_PIXELS, RENDERER_VERSION, SCENE_VERSION};
use std::collections::{BTreeMap, HashMap};
use taffy::prelude::*;

pub(crate) fn build(
    input: &LayoutInput,
    tree: &TaffyTree<usize>,
    ids: &HashMap<usize, NodeId>,
    order: Vec<usize>,
    root_idx: usize,
    by_id: &HashMap<&str, usize>,
    source: &mut impl MeasurementSource,
) -> Result<LayoutStep, LayoutError> {
    let mut misses = BTreeMap::new();
    let mut nodes = Vec::new();
    let mut diagnostics = Vec::new();
    let mut positions: HashMap<usize, (f32, f32)> = HashMap::new();
    for idx in order {
        let n = &input.nodes[idx];
        let p = &n.props;
        let l = tree.layout(ids[&idx]).map_err(|e| err(e.to_string()))?;
        let parent_xy = if idx == root_idx {
            (0.0, 0.0)
        } else {
            n.parent_id
                .as_deref()
                .and_then(|s| by_id.get(s))
                .and_then(|i| positions.get(i))
                .copied()
                .unwrap_or_default()
        };
        let (x, y) = if idx == root_idx {
            (0.0, 0.0)
        } else {
            (parent_xy.0 + l.location.x, parent_xy.1 + l.location.y)
        };
        positions.insert(idx, (x, y));
        let mut ascent = 0.0;
        let lines = if n.kind == "text" {
            let req = request(n, &input.fonts, Some(l.size.width))?;
            if let Some(m) = source.get(&req)? {
                ascent = m.ascent;
                crate::text_lines::diagnostics(&mut diagnostics, n, m);
                crate::text_lines::align(m, p, quantize(l.size.width))
            } else {
                misses.insert(req.key.clone(), req);
                Vec::new()
            }
        } else {
            Vec::new()
        };
        nodes.push(SceneNode {
            id: n.id.clone(),
            effective_theme: p
                .effective_theme
                .clone()
                .unwrap_or_else(|| input.theme.clone()),
            parent_id: if idx == root_idx {
                None
            } else {
                n.parent_id.clone()
            },
            kind: n.kind.clone(),
            bounds: Bounds {
                x: quantize(x),
                y: quantize(y),
                width: quantize(l.size.width),
                height: quantize(l.size.height),
            },
            lines,
            font_id: p.font_id.clone().or_else(|| {
                if n.kind == "text" {
                    Some("font_bundled".into())
                } else {
                    None
                }
            }),
            font_size: p.font_size.unwrap_or(16.0),
            line_height: p.line_height.unwrap_or(24.0),
            letter_spacing: p.letter_spacing.unwrap_or(0.0),
            decoration: crate::text_lines::decoration(p, ascent),
            fill: p.fill.clone().unwrap_or_else(|| {
                if n.kind == "text" {
                    "#182230".into()
                } else {
                    "transparent".into()
                }
            }),
            asset_id: p.asset_id.clone(),
            fit: crate::scene_fit(p.fit.as_deref())
                .unwrap_or("contain")
                .into(),
            opacity: p.opacity.unwrap_or(1.0),
            clip: p.clip.unwrap_or(n.kind == "frame"),
            radius: p.radius.as_ref().map_or(Sides::All(0.0), |r| match r {
                Sides::All(v) => Sides::All(*v),
                _ => Sides::Each(r.each()),
            }),
            shadow: p.shadow.clone().unwrap_or_default(),
            crop: p.crop.clone(),
            stroke: p
                .stroke
                .as_ref()
                .filter(|_| n.kind != "text")
                .and_then(StrokeInput::scene),
        });
    }
    if !misses.is_empty() {
        return Ok(LayoutStep::Pending {
            requests: misses.into_values().collect(),
        });
    }
    let viewport = surface(input, &nodes[0], root_idx)?;
    let source_scene_key = hash(&(input, SCENE_VERSION, RENDERER_VERSION));
    Ok(LayoutStep::Ready {
        scene: Scene {
            schema_version: 1,
            coordinate_space: "frame-local".into(),
            provenance: Provenance {
                project_id: input.project_id.clone(),
                revision: input.revision,
                frame_id: input.frame_id.clone(),
                viewport,
                theme: input.theme.clone(),
                state: input.state.clone(),
                scene_version: SCENE_VERSION.into(),
                renderer_version: RENDERER_VERSION.into(),
                source_scene_key,
                font_manifest_hash: hash(&input.fonts),
                asset_manifest_hash: hash(&input.assets),
            },
            nodes,
            fonts: input.fonts.clone(),
            assets: input.assets.clone(),
            diagnostics: crate::text_lines::cap(diagnostics),
        },
    })
}

/// The painted surface: the input viewport, except that a hug root's computed height
/// replaces the viewport height, within the same budget.
fn surface(
    input: &LayoutInput,
    root: &SceneNode,
    root_idx: usize,
) -> Result<Viewport, LayoutError> {
    if !hugs_height(&input.nodes[root_idx]) {
        return Ok(input.viewport.clone());
    }
    let height = f64::from(root.bounds.height);
    if height > 100_000.0 || input.viewport.width * height > f64::from(MAX_PIXELS) {
        return Err(err("Hug root height exceeds the viewport budget"));
    }
    Ok(Viewport {
        width: input.viewport.width,
        height,
    })
}
