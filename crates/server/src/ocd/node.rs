//! One node line into `create_node`: its ID (written, or from `auto_id`), parent, props and
//! the defaults written for it. Every `comp` above it in the source records it as a master
//! descendant, so a later `use` of that comp can check its override targets.
use super::auto_id::Siblings;
use super::defaults::{self, Parent};
use super::error::Result;
use super::instance_keys;
use super::parser::{Kind, NodeStmt, Part};
use super::props::{self, Target};
use super::value;
use serde_json::{json, Value};
use std::collections::HashMap;

/// An open container: the node (or `@in` target) that deeper lines nest under.
pub struct Open {
    pub id: String,
    /// The nearest ID written with `#id` (or the `@in` parent) at or above this node.
    pub anchor: String,
    pub children: usize,
    pub siblings: Siblings,
    /// The container's layout, or `None` under `@in`, whose stored parent is not known here.
    pub parent: Option<Parent>,
    /// The `@in` line: its children name their own IDs, and it must have at least one.
    pub within: Option<super::lexer::Token>,
    pub comp: bool,
}

/// What the compiler knows across lines: used IDs (with their line), descendants of each
/// `comp`, and the IDs it generated (with their line).
#[derive(Default)]
pub struct Batch {
    pub ids: HashMap<String, usize>,
    pub masters: HashMap<String, Vec<String>>,
    pub auto_ids: Vec<(usize, String)>,
}

/// Compiles `node` under the innermost open container and opens it in turn.
pub fn create(node: NodeStmt, stack: &mut Vec<Open>, batch: &mut Batch) -> Result<Value> {
    let head = &node.head;
    match (stack.last(), node.kind) {
        (None, Kind::Page) if node.id.is_none() => {
            return Err(head.error(
                "A root page needs #id, e.g. page#home; a generated ID would collide with the next batch's",
            ))
        }
        (None, Kind::Page) => {}
        (None, _) => {
            return Err(head.error(
                "Only pages sit at the root; nest this under a page, or under @in <parentId>",
            ))
        }
        (Some(_), Kind::Page) => return Err(head.error("A page must not be indented")),
        (Some(open), _) if open.within.is_some() && node.id.is_none() => {
            return Err(head.error("Nodes directly under @in need #id"));
        }
        (Some(_), _) => {}
    }
    let (index, component_name, parts) = take_special(&node)?;
    let target = match node.kind {
        Kind::Use => Target::Instance,
        _ => Target::Node,
    };
    let text_key = if node.kind == Kind::Text {
        "text"
    } else {
        "name"
    };
    let mut props = props::build(&parts, target, Some(text_key))?;
    let written = node.id.as_ref().map(value::id).transpose()?;
    if let Some(component) = &node.component {
        let master = value::id(component)?;
        instance_keys::check(&parts, batch.masters.get(&master).map(Vec::as_slice))?;
        props.insert("componentId".into(), json!(master));
    }
    let parent = stack.last_mut();
    let parent_id = parent.as_ref().map(|open| open.id.clone());
    let id = match (&written, parent) {
        (Some(id), _) => id.clone(),
        (None, None) => unreachable!("a root page has #id"),
        (None, Some(open)) => {
            let taken = |id: &str| batch.ids.contains_key(id);
            let source_kind = head.text.split(['#', '.']).next().unwrap_or("node");
            let id = open
                .siblings
                .next(&open.anchor, &open.id, source_kind, &props, taken);
            batch.auto_ids.push((head.line, id.clone()));
            id
        }
    };
    let text_size = match stack.last_mut() {
        Some(open) => {
            open.children += 1;
            defaults::text_size(open.parent.as_ref())
        }
        None => (None, None),
    };
    if let Some(first) = batch.ids.insert(id.clone(), head.line) {
        let at = node.id.as_ref().unwrap_or(head);
        return Err(at.error(format!("Duplicate ID `{id}` (first used on line {first})")));
    }
    for open in stack.iter().filter(|open| open.comp) {
        batch
            .masters
            .entry(open.id.clone())
            .or_default()
            .push(id.clone());
    }
    match node.kind {
        Kind::Text => {
            let (width, height) = text_size;
            for (key, size) in [("width", width), ("height", height)] {
                if let Some(size) = size {
                    props.entry(key).or_insert(json!(size));
                }
            }
        }
        Kind::Comp => {
            let name = component_name.unwrap_or_else(|| id.clone());
            props.entry("component").or_insert(json!({ "name": name }));
        }
        _ => {}
    }
    let anchor = match (&written, stack.last()) {
        (None, Some(open)) => open.anchor.clone(),
        _ => id.clone(),
    };
    stack.push(Open {
        id: id.clone(),
        anchor,
        children: 0,
        siblings: Siblings::default(),
        parent: Some(Parent::of(&props)),
        within: None,
        comp: node.kind == Kind::Comp,
    });
    let mut op = json!({"op":"create_node","id":id,"kind":kind(node.kind),
        "parentId":parent_id,"props":props});
    if let Some(index) = index {
        op["index"] = index;
    }
    Ok(op)
}

/// Splits off `index=` (an operation field, not a prop) and a component's `name=`.
fn take_special(node: &NodeStmt) -> Result<(Option<Value>, Option<String>, Vec<Part>)> {
    let (mut index, mut name, mut parts) = (None, None, Vec::new());
    for part in &node.parts {
        match part {
            Part::Attr(key, raw) if key.text == "index" => index = Some(value::index(raw)?),
            Part::Attr(key, raw) if key.text == "name" && node.kind == Kind::Comp => {
                match value::literal(raw)? {
                    Value::String(text) => name = Some(text),
                    _ => {
                        return Err(
                            raw.error("A component name is text: name=Input or name=\"Sign in\"")
                        )
                    }
                }
            }
            _ => parts.push(part.clone()),
        }
    }
    Ok((index, name, parts))
}

fn kind(kind: Kind) -> &'static str {
    match kind {
        Kind::Page => "page",
        Kind::Frame | Kind::Comp => "frame",
        Kind::Group => "group",
        Kind::Rect => "rectangle",
        Kind::Text => "text",
        Kind::Image => "image",
        Kind::Use => "instance",
    }
}
