//! Advisory text checks. Never reject or rewrite authored content.
mod fragmentation;
mod intent;
mod overflow;
#[cfg(test)]
mod tests;
pub(crate) use fragmentation::authored;
pub(crate) use intent::overflow_ids;
pub(crate) use overflow::append;
