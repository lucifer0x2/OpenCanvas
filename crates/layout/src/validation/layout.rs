use crate::model::*;
use crate::stack::{ALIGNS, JUSTIFIES};
use crate::util::{err, finite, is_fill};
use crate::{MAX_BREAKPOINTS, percentage, valid_breakpoint_width};
use std::collections::HashMap;

type Axis = fn(&Props) -> &Option<Sizing>;

pub(super) fn properties(n: &AuthoredNode) -> Result<(), LayoutError> {
    let p = &n.props;
    for (min, max) in [(p.min_width, p.max_width), (p.min_height, p.max_height)] {
        if matches!((min, max), (Some(min), Some(max)) if min > max) {
            return Err(err("Minimum must not exceed maximum"));
        }
    }
    if let Some(layout) = &p.layout {
        if !is_container(n) {
            return Err(err("Only containers support layout"));
        }
        if layout.breakpoints.len() > MAX_BREAKPOINTS {
            return Err(err("layout.breakpoints supports at most 8 rules"));
        }
        let mut previous = 0;
        for breakpoint in &layout.breakpoints {
            if !valid_breakpoint_width(breakpoint.max_width, previous) {
                return Err(err(
                    "Breakpoint maxWidth must be strictly increasing in 1..100000",
                ));
            }
            previous = breakpoint.max_width;
        }
        for rule in layout.rules() {
            stack_rule(rule)?;
        }
    }
    if let Some(stroke) = &p.stroke {
        stroke.check()?;
    }
    Ok(())
}

pub(super) fn relationships(
    n: &AuthoredNode,
    input: &LayoutInput,
    ids: &HashMap<&String, &AuthoredNode>,
) -> Result<(), LayoutError> {
    let parent = |n: &AuthoredNode| n.parent_id.as_ref().and_then(|id| ids.get(id)).copied();
    if wraps(n) && !definite_axis(Some(n), input, ids, |p| &p.width, true) {
        return Err(err("layout.wrap requires definite positive width"));
    }
    // A nested frame rendered directly is sized by the viewport too.
    if n.id == input.frame_id {
        return Ok(());
    }
    let axes: [Axis; 2] = [|p| &p.width, |p| &p.height];
    for axis in axes {
        if matches!(axis(&n.props), Some(Sizing::Rule(r)) if percentage(r).is_some())
            && !definite_axis(parent(n), input, ids, axis, false)
        {
            return Err(err("Percentage requires a definite parent axis"));
        }
    }
    if parent(n).is_some_and(wraps)
        && is_fill(&n.props.width)
        && n.props.min_width.is_none_or(|v| v <= 0.0)
    {
        return Err(err(
            "Wrapping row fill child requires explicit positive minWidth",
        ));
    }
    Ok(())
}

fn wraps(n: &AuthoredNode) -> bool {
    n.props
        .layout
        .as_ref()
        .is_some_and(|l| l.rules().any(|r| r.wrap))
}

fn definite_axis<'a>(
    mut current: Option<&'a AuthoredNode>,
    input: &LayoutInput,
    ids: &HashMap<&String, &'a AuthoredNode>,
    axis: Axis,
    positive: bool,
) -> bool {
    // Tree shape and cycles have already been checked by the caller.
    while let Some(node) = current {
        if node.id == input.frame_id {
            return true;
        }
        match axis(&node.props) {
            Some(Sizing::Fixed(v)) => return !positive || *v > 0.0,
            Some(Sizing::Rule(r))
                if r == "fill" || percentage(r).is_some_and(|v| !positive || v > 0.0) => {}
            _ => return false,
        }
        current = node.parent_id.as_ref().and_then(|id| ids.get(id)).copied();
    }
    false
}

fn stack_rule(l: &LayoutRule) -> Result<(), LayoutError> {
    if !["row", "column", "free"].contains(&l.mode.as_str())
        || !l.padding.edges().is_some_and(|e| e.into_iter().all(finite))
        || !finite(l.gap)
    {
        return Err(err("Unsupported layout"));
    }
    if l.wrap && l.mode != "row" {
        return Err(err("layout.wrap requires a row container"));
    }
    let stack = matches!(l.mode.as_str(), "row" | "column");
    for (key, value, allowed) in [
        ("align", &l.align, &ALIGNS),
        ("justify", &l.justify, &JUSTIFIES),
    ] {
        match value.as_deref() {
            Some(_) if !stack => {
                return Err(err(format!("layout.{key} requires mode row or column")));
            }
            Some(v) if !allowed.contains(&v) => {
                return Err(err(format!("Unsupported layout.{key}")));
            }
            _ => {}
        }
    }
    Ok(())
}

fn is_container(n: &AuthoredNode) -> bool {
    ["page", "frame", "group"].contains(&n.kind.as_str())
}
