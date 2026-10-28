mod cache_hints;
mod domain;
mod error;
mod events;
mod fingerprint;
mod http;
mod inspect;
mod instructions;
mod jobs;
mod mcp;
mod props_doc;
mod reads;
mod render_response;
mod resources;
mod schemas;
mod state;
mod worker;
mod workspace_events;

pub use error::ApiError;
pub use http::router;
pub use state::{AppState, Config};

/// Names of the published MCP tools in `tools/list` order, so tests and tooling compare against
/// the catalog instead of a hardcoded count.
pub fn tool_names() -> Vec<String> {
    schemas::tools()
        .into_iter()
        .map(|t| t.name.to_string())
        .collect()
}
