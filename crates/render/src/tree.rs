//! Scene indexing and conservative logical paint extents, matching renderer/scene-tree.
use crate::{geometry::Rect, resources::Prepared, shadow, stroke, PaintScene, RenderError};
use opencanvas_layout::SceneNode;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Tree {
    pub roots: Vec<usize>,
    pub children: Vec<Vec<usize>>,
    pub units: Vec<usize>,
    pub extents: Vec<Option<Rect>>,
}
impl Tree {
    pub fn new(s: &PaintScene, r: &Prepared) -> Result<Self, RenderError> {
        let mut ids = BTreeMap::new();
        for (i, n) in s.nodes.iter().enumerate() {
            if ids.insert(&n.id, i).is_some() {
                return Err(RenderError::scene("Duplicate scene node id"));
            }
        }
        let mut t = Self {
            roots: Vec::new(),
            children: vec![Vec::new(); s.nodes.len()],
            units: vec![0; s.nodes.len()],
            extents: vec![None; s.nodes.len()],
        };
        for (i, n) in s.nodes.iter().enumerate() {
            if let Some(p) = n.parent_id.as_ref().and_then(|id| ids.get(id)) {
                t.children[*p].push(i);
            } else {
                t.roots.push(i);
            }
        }
        let mut visited = BTreeSet::new();
        for i in t.roots.clone() {
            t.visit(i, s, r, 0, &mut visited)?;
        }
        if visited.len() != s.nodes.len() {
            return Err(RenderError::scene("Scene cycle"));
        }
        Ok(t)
    }
    fn visit(
        &mut self,
        i: usize,
        s: &PaintScene,
        r: &Prepared,
        depth: usize,
        visited: &mut BTreeSet<usize>,
    ) -> Result<(), RenderError> {
        if depth > 128 || !visited.insert(i) {
            return Err(RenderError::scene("Scene depth or cycle"));
        }
        let n = &s.nodes[i];
        let stroke = stroke::edges(n);
        let own = n.kind == "text" || n.kind == "image" || n.fill.as_solid() != "transparent";
        // Gradients paint even though Paint::as_solid returns transparent for them.
        let own = own || matches!(n.fill, opencanvas_layout::Paint::Gradient(_));
        self.units[i] = usize::from(own)
            + usize::from(text_overlap(n))
            + usize::from(stroke.is_some())
            + n.shadow.len();
        let mut extent = if own { Some(own_extent(n, r)) } else { None };
        if let Some(e) = stroke {
            extent = union(extent, Some(e.outer));
        }
        extent = union(extent, shadow::extent(n));
        for child in self.children[i].clone() {
            self.visit(child, s, r, depth + 1, visited)?;
            let mut inner = self.extents[child];
            if inner.is_some() {
                self.units[i] += 1;
            }
            if n.clip {
                inner = inner.and_then(|b| b.intersect(Rect::from_bounds(&n.bounds)));
            }
            extent = union(extent, inner);
        }
        self.extents[i] = extent;
        Ok(())
    }
}
fn union(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.union(b)),
        (a, b) => a.or(b),
    }
}
fn own_extent(n: &SceneNode, r: &Prepared) -> Rect {
    let b = Rect::from_bounds(&n.bounds);
    if (n.kind == "text" && !n.font_id.as_ref().is_some_and(|f| r.fonts.get(f).is_some()))
        || (n.kind == "image"
            && !n
                .asset_id
                .as_ref()
                .is_some_and(|a| r.images.contains_key(a)))
    {
        return Rect {
            x: f32::NEG_INFINITY,
            y: f32::NEG_INFINITY,
            w: f32::INFINITY,
            h: f32::INFINITY,
        };
    }
    if n.kind != "text" {
        return b;
    }
    let mut extent = b.grow(n.font_size.max(n.line_height));
    for l in &n.lines {
        extent = extent.union(Rect {
            x: b.x + l.x - n.font_size,
            y: b.y + l.y - 2.0 * n.font_size,
            w: l.width + 2.0 * n.font_size,
            h: 3.0 * n.font_size,
        });
        if let Some(ink) = &l.ink_bounds {
            extent = extent.union(Rect {
                x: b.x + l.x + ink.x,
                y: b.y + l.y + ink.y,
                w: ink.width,
                h: ink.height,
            });
        }
        if let Some(d) = &n.decoration {
            extent = extent.union(Rect {
                x: b.x + l.x,
                y: (b.y + l.y + d.offset + 0.5).floor(),
                w: l.width,
                h: d.thickness,
            });
        }
    }
    extent
}
fn text_overlap(n: &SceneNode) -> bool {
    if n.kind != "text" || n.lines.is_empty() {
        return false;
    }
    if n.decoration.is_some() || n.letter_spacing < 0.0 {
        return true;
    }
    let mut bottom = f32::NEG_INFINITY;
    for l in &n.lines {
        let Some(ink) = &l.ink_bounds else {
            return n.lines.len() > 1;
        };
        let top = l.y + ink.y;
        if top < bottom + 1.0 {
            return true;
        }
        bottom = bottom.max(top + ink.height);
    }
    false
}
