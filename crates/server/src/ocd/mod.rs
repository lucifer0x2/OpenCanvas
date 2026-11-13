//! OCD v1: a line DSL that `apply_batch` accepts as `source` and compiles into the ordinary
//! JSON `operations` array before schema validation. Nothing here touches storage: the compiled
//! batch is validated and committed exactly like an inline one. Grammar and shorthands are
//! published in `skill://opencanvas/ocd.md`.
mod apply;
mod defaults;
mod directives;
mod error;
mod expand;
mod keys;
mod lexer;
mod parser;
mod props;
mod set;
mod value;

pub use apply::{compile_into, Positions};
pub use error::OcdError;
use opencanvas_storage::MAX_BATCH_BYTES;
use serde_json::Value;

/// The `operations` limit of `apply_batch`; a longer source fails while parsing, at the line
/// that crosses it, instead of with a generic schema error.
const MAX_OPERATIONS: usize = 500;

/// Compiled operations and, for each, the 1-based `(line, column)` of the statement that
/// produced it. Positions are for error reports only; they are not part of the batch.
#[derive(Debug, Default)]
pub struct Compiled {
    pub operations: Vec<Value>,
    pub positions: Vec<(usize, usize)>,
}

/// Compiles OCD source into operations.
pub fn compile(source: &str) -> Result<Compiled, OcdError> {
    if source.len() > MAX_BATCH_BYTES {
        return Err(too_long(source));
    }
    let lines = lexer::lines(source)?;
    if lines.is_empty() {
        return Err(OcdError::new(1, 1, "Source has no nodes or directives"));
    }
    let statements = lines
        .into_iter()
        .map(|line| Ok((line.depth, parser::statement(line)?)))
        .collect::<Result<Vec<_>, OcdError>>()?;
    let compiled = expand::operations(statements)?;
    if compiled.operations.is_empty() {
        return Err(OcdError::new(1, 1, "Source compiles to no operations"));
    }
    Ok(compiled)
}

/// The 2 MiB cap counts UTF-8 bytes; the error points at the character that crosses it.
fn too_long(source: &str) -> OcdError {
    let mut cut = MAX_BATCH_BYTES;
    while !source.is_char_boundary(cut) {
        cut -= 1;
    }
    let before = &source[..cut];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .chars()
        .count()
        + 1;
    let bytes = source.len();
    OcdError::new(
        line,
        column,
        format!("Source is {bytes} UTF-8 bytes; the limit is {MAX_BATCH_BYTES} (2 MiB): split it into batches"),
    )
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_errors;
#[cfg(test)]
mod tests_rules;
#[cfg(test)]
mod tests_set;
#[cfg(test)]
mod tests_shorthands;
