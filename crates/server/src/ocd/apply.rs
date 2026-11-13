//! `apply_batch` with `source`: the compiled operations replace `source`, then the batch is
//! checked like an inline one. Errors about a compiled operation (the published schema, the
//! batch byte cap, storage errors with `opIndex`) gain the `line` and `column` of the
//! statement that produced it and keep their own code and fields.
use super::{compile, Compiled, OcdError};
use crate::error::ApiError;
use opencanvas_storage::{BatchRequest, MAX_BATCH_BYTES};
use serde_json::{json, Value};

/// Statement positions of the compiled operations, by operation index.
#[derive(Debug)]
pub struct Positions(Vec<(usize, usize)>);

impl Positions {
    /// Adds the statement position to an error that names an operation with `opIndex`.
    pub fn locate(&self, error: ApiError) -> ApiError {
        match error.0.op_index {
            Some(index) => self.at(index, error),
            None => error,
        }
    }
    fn at(&self, index: usize, error: ApiError) -> ApiError {
        match self.0.get(index) {
            Some(&(line, column)) => error
                .with_detail("line", json!(line))
                .with_detail("column", json!(column)),
            None => error,
        }
    }
}

/// Compiles `source` into `args["operations"]` and validates the request. `pages` is the
/// request's `pageIds`, which count toward the batch byte cap.
pub fn compile_into(
    args: &mut Value,
    source: &str,
    pages: Option<&Value>,
) -> Result<Positions, ApiError> {
    let Compiled {
        operations,
        positions,
    } = compile(source)?;
    let positions = Positions(positions);
    args["operations"] = Value::Array(operations);
    if let Err(error) = crate::schemas::validate("apply_batch", args) {
        let operations = args["operations"].as_array().map_or(&[][..], Vec::as_slice);
        return Err(match crate::schemas::invalid_operation(operations) {
            Some(index) => positions.at(index, error),
            None => error,
        });
    }
    fits(args, pages, &positions)?;
    Ok(positions)
}

/// Storage caps the canonical request (typed operations plus `pageIds`) at 2 MiB; a source
/// that compiles past it fails here, at the statement whose operation crosses the cap.
fn fits(args: &Value, pages: Option<&Value>, positions: &Positions) -> Result<(), OcdError> {
    // A request storage cannot decode fails later with its usual error.
    let Ok(request) = serde_json::from_value::<BatchRequest>(args.clone()) else {
        return Ok(());
    };
    let pages = pages.map_or(0, |p| r#","pageIds":"#.len() + p.to_string().len());
    let total = json_len(&request) + pages;
    if total <= MAX_BATCH_BYTES {
        return Ok(());
    }
    // Each operation plus its comma; the rest is the request envelope.
    let sizes: Vec<usize> = request
        .operations
        .iter()
        .map(|op| json_len(op) + 1)
        .collect();
    let mut used = total.saturating_sub(sizes.iter().sum());
    let index = sizes
        .iter()
        .position(|bytes| {
            used += bytes;
            used > MAX_BATCH_BYTES
        })
        .unwrap_or(sizes.len().saturating_sub(1));
    let (line, column) = positions.0.get(index).copied().unwrap_or((1, 1));
    Err(OcdError::new(
        line,
        column,
        format!("Source compiles to a {total}-byte batch, over the {MAX_BATCH_BYTES}-byte (2 MiB) cap from this statement on; split it into batches"),
    ))
}

fn json_len<T: serde::Serialize>(value: &T) -> usize {
    serde_json::to_vec(value).map_or(0, |bytes| bytes.len())
}
