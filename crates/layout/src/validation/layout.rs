use crate::model::*;
use crate::util::{err, is_fill};
use std::collections::HashMap;

pub(super) fn properties(n: &AuthoredNode) -> Result<(), LayoutError> {
    let p = &n.props;
    for (min, max) in [(p.min_width, p.max_width), (p.min_height, p.max_height)] {
        if matches!((min, max), (Some(min), Some(max)) if min > max) {
            return Err(err("Minimum must not exceed maximum"));
        }
    }
    if p.layout.as_ref().is_some_and(|l| l.wrap)
        && (!is_container(n) || p.layout.as_ref().unwrap().mode != "row")
    {
        return Err(err("layout.wrap requires a row container"));
    }
    Ok(())
}

pub(super) fn relationships(
    n: &AuthoredNode,
    input: &LayoutInput,
    ids: &HashMap<&String, &AuthoredNode>,
) -> Result<(), LayoutError> {
    if n.props.layout.as_ref().is_some_and(|l| l.wrap) {
        let mut current = n;
        // Tree shape/cycles have already been checked by the caller.
        while current.id != input.frame_id {
            match &current.props.width {
                Some(Sizing::Fixed(v)) if *v > 0.0 => break,
                Some(Sizing::Rule(r)) if r == "fill" => {
                    current = current
                        .parent_id
                        .as_ref()
                        .and_then(|id| ids.get(id))
                        .copied()
                        .ok_or_else(|| err("layout.wrap requires definite width"))?;
                }
                _ => return Err(err("layout.wrap requires definite positive width")),
            }
        }
    }
    if let Some(parent) = n.parent_id.as_ref().and_then(|id| ids.get(id))
        && parent.props.layout.as_ref().is_some_and(|l| l.wrap)
        && is_fill(&n.props.width)
        && n.props.min_width.is_none_or(|v| v <= 0.0)
    {
        return Err(err(
            "Wrapping row fill child requires explicit positive minWidth",
        ));
    }
    Ok(())
}

fn is_container(n: &AuthoredNode) -> bool {
    ["page", "frame", "group"].contains(&n.kind.as_str())
}
