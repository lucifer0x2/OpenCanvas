//! Advisory text checks. Never reject or rewrite authored content.
mod budget;
mod fragmentation;
mod intent;
mod overflow;
#[cfg(test)]
mod tests;
pub(crate) use budget::{fit, fit_in_place, is_overflow, BUDGET_SHARE};
pub(crate) use fragmentation::authored;
pub(crate) use intent::overflow_ids;
pub(crate) use overflow::append;
