//! Bounded text advisories shared by native jobs and browser scene JSON.
//! These checks never reject or rewrite authored content and never include text.
mod intent;
mod overflow;

pub use intent::{overflow_ids, Intent};
pub use overflow::append;
