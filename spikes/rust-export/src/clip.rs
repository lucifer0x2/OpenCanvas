//! Whole-canvas clip masks for clipping nodes and images. `Full` keeps the original
//! `clone` + `Mask::intersect_path`; `Bbox` rasterizes the node into a fresh mask and multiplies the
//! inherited clip in with the same per-byte product, in parallel pieces, without the clone.
use crate::mode::PaintMode;
use crate::par;
use crate::scene::Node;
use crate::shape::{mask_product, node_path};
use tiny_skia::{FillRule, Mask, Transform};

/// `n`'s rounded bounds at scale `k`, intersected with `clip`, on a `w`×`h` mask.
pub fn mask(w: u32, h: u32, n: &Node, clip: Option<&Mask>, k: f32, mode: PaintMode) -> Mask {
    let path = node_path(n).unwrap();
    let tf = Transform::from_scale(k, k);
    if let (Some(parent), PaintMode::Full) = (clip, mode) {
        let mut m = parent.clone();
        m.intersect_path(&path, FillRule::Winding, true, tf);
        return m;
    }
    let mut m = Mask::new(w, h).unwrap();
    m.fill_path(&path, FillRule::Winding, true, tf);
    if let Some(parent) = clip {
        // intersect_path stores product(parent, own) per byte; same here, piece by piece.
        par::zip_chunks(m.data_mut(), par::CHUNK, parent.data(), par::CHUNK, |own, inherited| {
            for (o, i) in own.iter_mut().zip(inherited) {
                *o = mask_product(*i, *o);
            }
        });
    }
    m
}
