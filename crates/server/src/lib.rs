mod backend;
mod cache_hints;
mod domain;
mod error;
mod event_visibility;
mod events;
mod export;
mod fingerprint;
mod fonts;
mod http;
mod inspect;
mod instructions;
mod jobs;
mod mcp;
mod node_rows;
mod projects;
mod props_doc;
mod public_viewer;
mod reads;
mod render_response;
mod resources;
mod scenes;
mod schemas;
mod snapshot_http;
mod state;
mod svg;
mod text_diagnostics;
mod viewer_auth;
mod viewer_origin;
mod viewer_privacy;
mod worker;
mod workspace_events;

pub use backend::RenderBackend;
pub use error::ApiError;
pub use http::router;
pub use public_viewer::PublicViewer;
pub use state::{AppState, Config};

/// Names of the published MCP tools in `tools/list` order, so tests and tooling compare against
/// the catalog instead of a hardcoded count.
pub fn catalog_tool_names() -> Vec<String> {
    schemas::tools()
        .into_iter()
        .map(|t| t.name.to_string())
        .collect()
}
