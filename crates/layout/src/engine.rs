use crate::measurement::{
    CachedMeasurements, MeasurementSource, SynchronousMeasurements, TextMeasurer,
};
use crate::model::*;
use crate::tree::build;
use crate::util::*;
use crate::validation::validate;
use std::collections::{BTreeMap, HashMap};
use taffy::prelude::*;

/// Never performs RPC and never returns a computed scene while measurement misses remain.
pub fn compute(input: &LayoutInput, cache: &MeasureCache) -> Result<LayoutStep, LayoutError> {
    compute_with_source(input, &mut CachedMeasurements(cache), false)
}

/// Computes a complete scene with one engine invocation and prepared text resources.
///
/// Measurements are memoized by request key for this call only. Taffy may ask for
/// several widths per text; the final-width line lookup uses the same provider.
/// Provider errors retain their code/message and never produce a ready scene.
pub fn compute_with_measurer<M: TextMeasurer + ?Sized>(
    input: &LayoutInput,
    measurer: &mut M,
) -> Result<Scene, LayoutError> {
    let mut source = SynchronousMeasurements {
        measurer,
        cache: MeasureCache::new(),
    };
    match compute_with_source(input, &mut source, true)? {
        LayoutStep::Ready { scene } => Ok(scene),
        LayoutStep::Pending { .. } => Err(err("Synchronous measurement left unresolved requests")),
    }
}

fn compute_with_source(
    input: &LayoutInput,
    source: &mut impl MeasurementSource,
    stop_after_callback_error: bool,
) -> Result<LayoutStep, LayoutError> {
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
            width: AvailableSpace::Definite(input.viewport.width as f32),
            height: if hugs_height(&input.nodes[root_idx]) {
                AvailableSpace::MaxContent
            } else {
                AvailableSpace::Definite(input.viewport.height as f32)
            },
        },
        |inputs, _, context, style| {
            taffy::compute_leaf_layout(
                inputs,
                style,
                |_, _| 0.0,
                |known, available| {
                    // Preserve cache-path error selection; synchronous providers must
                    // never be queried again after the first callback failure.
                    if stop_after_callback_error && callback_error.is_some() {
                        return Size::ZERO;
                    }
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
                    match source.get(&req) {
                        Ok(Some(measure)) => Size {
                            width: known.width.unwrap_or(measure.width),
                            height: known.height.unwrap_or(measure.height),
                        },
                        Ok(None) => {
                            misses.insert(req.key.clone(), req);
                            Size {
                                width: known.width.unwrap_or(0.0),
                                height: known.height.unwrap_or(0.0),
                            }
                        }
                        Err(error) => {
                            callback_error = Some(error);
                            Size::ZERO
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
    crate::scene_builder::build(input, &tree, &ids, order, root_idx, &by_id, source)
}
