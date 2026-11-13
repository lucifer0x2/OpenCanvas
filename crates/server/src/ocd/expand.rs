//! Turns parsed lines into operations in source order: nesting gives `parentId`, nodes
//! without `#id` get `<parentId>-<position>`, and the written defaults are applied here.
//! Each operation keeps the position of the statement that produced it.
use super::defaults::{self, Parent};
use super::directives;
use super::error::{OcdError, Result};
use super::lexer::Token;
use super::parser::{DirectiveStmt, Kind, NodeStmt, Part, Stmt};
use super::props::{self, Target};
use super::{value, Compiled, MAX_OPERATIONS};
use serde_json::{json, Value};
use std::collections::HashMap;

/// An open container: the node (or `@in` target) that deeper lines nest under.
struct Open {
    id: String,
    children: usize,
    /// The container's layout, or `None` under `@in`, whose stored parent is not known here.
    parent: Option<Parent>,
    /// The `@in` line: its children name their own IDs (auto IDs could collide with
    /// existing children), and it must have at least one.
    within: Option<Token>,
}

impl Open {
    fn close(self) -> Result<()> {
        match self.within {
            // `at` is the name after `@`; point at the `@`.
            Some(at) if self.children == 0 => Err(OcdError::new(
                at.line,
                at.col - 1,
                "@in needs indented child nodes",
            )),
            _ => Ok(()),
        }
    }
}

pub fn operations(statements: Vec<(usize, Stmt)>) -> Result<Compiled> {
    let mut out = Compiled::default();
    let mut stack: Vec<Open> = Vec::new();
    let mut ids: HashMap<String, usize> = HashMap::new();
    // `None` before the first line; then the previous directive's name, if it was one
    // that takes no children.
    let mut previous: Option<Option<String>> = None;
    for (depth, statement) in statements {
        let at = match &statement {
            Stmt::Node(node) => &node.head,
            Stmt::Directive(directive) => &directive.name,
        };
        let (line, column) = (at.line, depth * 2 + 1);
        if depth > stack.len() {
            let message = match &previous {
                None => "The first line must not be indented".to_owned(),
                Some(Some(name)) => {
                    format!(
                        "Indented under @{name}, which takes no children; only nodes and @in do"
                    )
                }
                Some(None) => "Indented deeper than one level below the line above".to_owned(),
            };
            return Err(OcdError::new(line, column, message));
        }
        for open in stack.drain(depth..) {
            open.close()?;
        }
        previous = Some(match &statement {
            Stmt::Directive(d) if d.name.text != "in" => Some(d.name.text.clone()),
            _ => None,
        });
        let op = match statement {
            Stmt::Directive(d) if d.name.text == "in" => {
                stack.push(within(d)?);
                continue;
            }
            Stmt::Directive(d) => directives::operations(d)?,
            Stmt::Node(node) => {
                let (op, open) = create(node, stack.last_mut(), &mut ids)?;
                stack.push(open);
                op
            }
        };
        out.operations.push(op);
        out.positions.push((line, column));
        if out.operations.len() > MAX_OPERATIONS {
            return Err(OcdError::new(
                line,
                column,
                format!("Source compiles to more than {MAX_OPERATIONS} operations; split it into batches"),
            ));
        }
    }
    for open in stack {
        open.close()?;
    }
    Ok(out)
}

/// `@in parentId`: exactly one ID; the new nodes go on the indented lines below it.
fn within(d: DirectiveStmt) -> Result<Open> {
    if let Some(part) = d.parts.first() {
        let at = directives::token_of(part);
        return Err(at.error("@in takes only a parent ID; write its children on indented lines"));
    }
    Ok(Open {
        id: value::id(&d.positional[0])?,
        children: 0,
        parent: None,
        within: Some(d.name),
    })
}

fn create(
    node: NodeStmt,
    parent: Option<&mut Open>,
    ids: &mut HashMap<String, usize>,
) -> Result<(Value, Open)> {
    let head = &node.head;
    let (parent_id, id, text_size) = match (parent, node.kind) {
        (None, Kind::Page) => {
            let Some(id) = &node.id else {
                return Err(head.error(
                    "A root page needs #id, e.g. page#home; a generated ID would collide with the next batch's",
                ));
            };
            (None, value::id(id)?, (None, None))
        }
        (None, _) => {
            return Err(head.error(
                "Only pages sit at the root; nest this under a page, or under @in <parentId>",
            ))
        }
        (Some(_), Kind::Page) => return Err(head.error("A page must not be indented")),
        (Some(open), _) => {
            if open.within.is_some() && node.id.is_none() {
                return Err(head.error("Nodes directly under @in need #id"));
            }
            open.children += 1;
            let id = match &node.id {
                Some(token) => value::id(token)?,
                None => format!("{}-{}", open.id, open.children),
            };
            let size = defaults::text_size(open.parent.as_ref());
            (Some(open.id.clone()), id, size)
        }
    };
    if let Some(first) = ids.insert(id.clone(), head.line) {
        let at = node.id.as_ref().unwrap_or(head);
        return Err(at.error(format!("Duplicate ID `{id}` (first used on line {first})")));
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
        Kind::Use => {
            let component = node.component.as_ref().expect("parser requires it");
            props.insert("componentId".into(), json!(value::id(component)?));
        }
        _ => {}
    }
    let open = Open {
        id: id.clone(),
        children: 0,
        parent: Some(Parent::of(&props)),
        within: None,
    };
    let mut op = json!({"op":"create_node","id":id,"kind":kind(node.kind),
        "parentId":parent_id,"props":props});
    if let Some(index) = index {
        op["index"] = index;
    }
    Ok((op, open))
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
