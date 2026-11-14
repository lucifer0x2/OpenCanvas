use crate::model::*;
use crate::stack::{self, Parent};
use crate::util::{dim, err, is_fill, root_hugs_height};
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
    let has_breakpoints = p.layout.as_ref().is_some_and(|l| !l.breakpoints.is_empty());
    let layout = p.layout.as_ref().map(|l| l.at_width(input.viewport.width));
    let mode = layout.map(|l| l.mode.as_str()).unwrap_or("free");
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
    if let Some(layout) = layout {
        // An inside stroke takes space in a stack like a CSS border; free children keep x/y.
        let insets = match (&p.stroke, mode) {
            (Some(stroke), "row" | "column") => stroke.inside_insets(),
            _ => [0.0; 4],
        };
        stack::apply(&mut style, layout, insets);
    }
    let hugs = root && root_hugs_height(p, input.viewport.width);
    if root {
        style.size = Size {
            width: length(input.viewport.width as f32),
            height: if hugs {
                auto()
            } else {
                length(input.viewport.height as f32)
            },
        };
        if hugs {
            // A paintable surface: an empty hug root is still one pixel tall.
            style.min_size.height = length(p.min_height.unwrap_or(0.0).max(1.0));
        }
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
    // Taffy's static boxes pass absolute descendants up to a containing block.
    // Anchor responsive free children to their immediate parent, also when a
    // breakpoint changes this container's placement from absolute to in-flow.
    if style.position == Position::Static
        && mode == "free"
        && (has_breakpoints
            || parent.has_breakpoints
            || children.iter().any(|&i| {
                let child = &input.nodes[i].props;
                [&child.width, &child.height]
                    .into_iter()
                    .any(|size| size.as_ref().and_then(Sizing::percentage).is_some())
            }))
    {
        style.position = Position::Relative;
    }
    let mut child_ids = Vec::new();
    let this = Parent {
        mode,
        align: stack::align(layout),
        has_breakpoints,
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
