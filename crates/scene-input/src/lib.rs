//! Pure authored models, validation, expansion and scene input preparation.
//! Default features require no database, filesystem, network or async runtime.
mod animation_catalog;
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
pub mod scene_diagnostics;
pub mod validate;

pub use animation_catalog::{AnimationKey, ANIMATED_PROPS, ANIMATION_KEYS, KEYFRAME_KEYS};
pub use error::{NodeErrorContext, Result, StorageError};
pub use export_settings::{
    check_export_settings, export_format, export_suffix, EXPORT_FORMATS, MAX_EXPORT_SETTINGS,
};
pub use image_fit::IMAGE_FITS;
pub use input::{make_input, SceneOptions};
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
