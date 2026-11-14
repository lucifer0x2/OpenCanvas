//! Pure authored models, validation, expansion and scene input preparation.
//! Default features require no database, filesystem, network or async runtime.
mod animation_catalog;
pub mod bench_phase;
mod content_key;
mod error;
mod export_settings;
mod image_fit;
mod input;
mod instance_paint;
pub mod instances;
mod links;
mod model;
mod props_catalog;
mod props_suggest;
#[path = "scoped/mod.rs"]
pub mod read_scope;
pub mod scene_diagnostics;
pub mod validate;
mod viewport;

pub use animation_catalog::{AnimationKey, ANIMATED_PROPS, ANIMATION_KEYS, KEYFRAME_KEYS};
pub use content_key::{frame_content_bases, frame_content_basis, CONTENT_BASIS_VERSION};
pub use error::{NodeErrorContext, Result, StorageError};
pub use export_settings::{
    check_export_settings, export_format, export_suffix, EXPORT_FORMATS, MAX_EXPORT_SETTINGS,
};
pub use image_fit::IMAGE_FITS;
pub use input::{frame_input, make_input, make_input_with_scale, ResolvedNodes, SceneOptions};
pub use links::{
    flow_start, node_links, Link, LINK_ACTIONS, LINK_KEYS, LINK_TRIGGERS, MAX_FLOW_NAME,
};
pub use model::*;
pub use props_catalog::{
    allowed_props, prop_spec, required_props, PropScope, PropSpec, NODE_KINDS, PROPS,
};
pub use props_suggest::suggest_prop;
pub use validate::{
    referenced_font_ids, resolved_expanded_nodes, resolved_nodes, resolved_nodes_for_theme,
    resolved_tokens_for_theme, validate_slug,
};
pub use viewport::check_viewport;
