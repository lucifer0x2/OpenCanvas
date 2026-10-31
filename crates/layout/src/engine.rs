use crate::model::*;
use crate::tree::build;
use crate::util::*;
use crate::validation::validate;
use crate::{RENDERER_VERSION, SCENE_VERSION};
use std::collections::{BTreeMap, HashMap};
use taffy::prelude::*;

/// Never performs RPC and never returns a computed scene while measurement misses remain.
pub fn compute(input: &LayoutInput, cache: &MeasureCache) -> Result<LayoutStep, LayoutError> {
    validate(input)?;
    let mut tree: TaffyTree<usize> = TaffyTree::new();
    tree.disable_rounding();
    let by_id: HashMap<&str, usize> = input
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let root_idx = *by_id
        .get(input.frame_id.as_str())
        .ok_or_else(|| err("Frame does not exist"))?;
    let mut ids = HashMap::new();
    let mut order = Vec::new();
    let root = build(
        root_idx,
        input,
        &mut tree,
        &mut ids,
        &mut order,
        crate::stack::Parent::ROOT,
        true,
    )?;
    let mut misses = BTreeMap::new();
    let mut callback_error = None;
    tree.compute_layout_with_measure(
        root,
        Size {
            width: AvailableSpace::Definite(input.viewport.width),
            height: AvailableSpace::Definite(input.viewport.height),
        },
        |inputs, _, context, style| {
            taffy::compute_leaf_layout(
                inputs,
                style,
                |_, _| 0.0,
                |known, available| {
                    let Some(idx) = context else {
                        return Size::ZERO;
                    };
                    let width = known.width.or(match available.width {
                        AvailableSpace::Definite(w) => Some(w),
                        AvailableSpace::MinContent => Some(0.0),
                        _ => None,
                    });
                    let req = match request(&input.nodes[*idx], &input.fonts, width) {
                        Ok(r) => r,
                        Err(e) => {
                            callback_error = Some(e);
                            return Size::ZERO;
                        }
                    };
                    if let Some(measure) = cache.get(&req.key) {
                        Size {
                            width: known.width.unwrap_or(measure.width),
                            height: known.height.unwrap_or(measure.height),
                        }
                    } else {
                        misses.insert(req.key.clone(), req);
                        Size {
                            width: known.width.unwrap_or(0.0),
                            height: known.height.unwrap_or(0.0),
                        }
                    }
                },
            )
        },
    )
    .map_err(|e| err(e.to_string()))?;
    if let Some(e) = callback_error {
        return Err(e);
    }
    if !misses.is_empty() {
        return Ok(LayoutStep::Pending {
            requests: misses.into_values().collect(),
        });
    }
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
            if let Some(m) = cache.get(&req.key) {
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
    let source_scene_key = hash(&(input, SCENE_VERSION, RENDERER_VERSION));
    Ok(LayoutStep::Ready {
        scene: Scene {
            schema_version: 1,
            coordinate_space: "frame-local".into(),
            provenance: Provenance {
                project_id: input.project_id.clone(),
                revision: input.revision,
                frame_id: input.frame_id.clone(),
                viewport: input.viewport.clone(),
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
