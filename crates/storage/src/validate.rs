mod candidate;
mod node;
mod properties;
mod refs;
mod values;
pub(crate) use candidate::Candidate;
pub use refs::resolved_nodes;
pub use values::{valid_id, validate_slug};
