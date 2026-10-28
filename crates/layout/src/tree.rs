use crate::model::*;
use crate::stack::{self, Parent};
use crate::util::{dim, err, is_fill};
use std::collections::HashMap;
use taffy::prelude::*;

pub(crate) fn build(
    idx: usize,
    input: &LayoutInput,
    tree: &mut TaffyTree<usize>,
    ids: &mut HashMap<usize, NodeId>,
    order: &mut Vec<usize>,
    parent: Parent,
    root: bool,
) -> Result<NodeId, LayoutError> {
    let n = &input.nodes[idx];
    let p = &n.props;
    let parent_mode = parent.mode;
    let mode = p.layout.as_ref().map(|l| l.mode.as_str()).unwrap_or("free");
    let mut style = Style {
        display: Display::Flex,
        flex_direction: if mode == "row" {
            FlexDirection::Row
        } else {
            FlexDirection::Column
        },
        align_items: AlignItems::START,
        align_self: stack::self_align(p, parent),
        flex_shrink: 0.0,
        size: Size {
            width: dim(&p.width, parent_mode != "row"),
            height: dim(&p.height, parent_mode != "column"),
        },
        min_size: Size {
            width: p.min_width.map_or(auto(), length),
            height: p.min_height.map_or(auto(), length),
        },
        max_size: Size {
            width: p.max_width.map_or(auto(), length),
            height: p.max_height.map_or(auto(), length),
        },
        ..Default::default()
    };
    if (parent_mode == "row" && is_fill(&p.width))
        || (parent_mode == "column" && is_fill(&p.height))
    {
        style.flex_grow = 1.0;
        style.flex_shrink = 1.0;
        style.flex_basis = length(0.0);
    }
    if let Some(layout) = &p.layout {
        stack::apply(&mut style, layout, [0.0; 4]);
    }
    if root {
        style.size = Size {
            width: length(input.viewport.width),
            height: length(input.viewport.height),
        };
    } else if parent_mode == "free" {
        style.position = Position::Absolute;
        style.inset.left = length(p.x.unwrap_or(0.0));
        style.inset.top = length(p.y.unwrap_or(0.0));
    }
    order.push(idx);
    let children: Vec<_> = input
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, c)| c.parent_id.as_deref() == Some(n.id.as_str()))
        .map(|(i, _)| i)
        .collect();
    let mut child_ids = Vec::new();
    let this = Parent {
        mode,
        align: stack::align(p.layout.as_ref()),
    };
    for child in children {
        child_ids.push(build(child, input, tree, ids, order, this, false)?);
    }
    let id = if n.kind == "text" {
        tree.new_leaf_with_context(style, idx)
    } else {
        tree.new_with_children(style, &child_ids)
    }
    .map_err(|e| err(e.to_string()))?;
    ids.insert(idx, id);
    Ok(id)
}
