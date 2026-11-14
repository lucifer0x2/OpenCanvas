//! Turns parsed lines into operations in source order: nesting gives `parentId`, nodes
//! (`node`) get their IDs and written defaults, and directives their operations. Each
//! operation keeps the position of the statement that produced it.
use super::auto_id::Siblings;
use super::directives;
use super::error::{OcdError, Result};
use super::node::{self, Batch, Open};
use super::parser::{DirectiveStmt, Stmt};
use super::{value, Compiled, MAX_OPERATIONS};

fn close(open: Open) -> Result<()> {
    match open.within {
        // `at` is the name after `@`; point at the `@`.
        Some(at) if open.children == 0 => Err(OcdError::new(
            at.line,
            at.col - 1,
            "@in needs indented child nodes",
        )),
        _ => Ok(()),
    }
}

pub fn operations(statements: Vec<(usize, Stmt)>) -> Result<Compiled> {
    let mut out = Compiled::default();
    let mut stack: Vec<Open> = Vec::new();
    let mut batch = Batch::default();
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
            close(open)?;
        }
        previous = Some(match &statement {
            Stmt::Directive(d) if d.name.text != "in" => Some(d.name.text.clone()),
            _ => None,
        });
        let ops = match statement {
            Stmt::Directive(d) if d.name.text == "in" => {
                stack.push(within(d)?);
                continue;
            }
            Stmt::Directive(d) => directives::operations(d)?,
            Stmt::Node(statement) => vec![node::create(statement, &mut stack, &mut batch)?],
        };
        for op in ops {
            out.operations.push(op);
            out.positions.push((line, column));
        }
        if out.operations.len() > MAX_OPERATIONS {
            return Err(OcdError::new(
                line,
                column,
                format!("Source compiles to more than {MAX_OPERATIONS} operations; split it into batches"),
            ));
        }
    }
    for open in stack {
        close(open)?;
    }
    out.auto_ids = batch.auto_ids;
    Ok(out)
}

/// `@in parentId`: exactly one ID; the new nodes go on the indented lines below it.
fn within(d: DirectiveStmt) -> Result<Open> {
    if let Some(part) = d.parts.first() {
        let at = directives::token_of(part);
        return Err(at.error("@in takes only a parent ID; write its children on indented lines"));
    }
    let id = value::id(&d.positional[0])?;
    Ok(Open {
        anchor: id.clone(),
        id,
        children: 0,
        siblings: Siblings::default(),
        parent: None,
        within: Some(d.name),
        comp: false,
    })
}
