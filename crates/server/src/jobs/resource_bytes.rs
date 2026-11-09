//! Job-owned original binaries. Native compute, PNG scales and SVG share these reads.
use crate::error::{ApiError, Result};
use opencanvas_storage::{Asset, Font};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(super) struct ResourceBytes {
    pub fonts: BTreeMap<String, (Font, String, Vec<u8>)>,
    pub assets: BTreeMap<String, (Asset, Vec<u8>)>,
}

impl ResourceBytes {
    pub fn font(&self, id: &str) -> Result<(Font, String, Vec<u8>)> {
        self.fonts.get(id).cloned().ok_or_else(unavailable)
    }
    pub fn asset(&self, id: &str) -> Result<(Asset, Vec<u8>)> {
        self.assets.get(id).cloned().ok_or_else(unavailable)
    }
    pub fn paint(&self) -> opencanvas_render::Resources<'_> {
        use opencanvas_render::ResourceBytes as Bytes;
        opencanvas_render::Resources {
            fonts: self
                .fonts
                .iter()
                .map(|(id, (font, _, bytes))| {
                    (
                        id.clone(),
                        Bytes {
                            hash: font.content_hash.as_deref().unwrap_or_default(),
                            bytes,
                        },
                    )
                })
                .collect(),
            assets: self
                .assets
                .iter()
                .map(|(id, (asset, bytes))| {
                    (
                        id.clone(),
                        Bytes {
                            hash: &asset.content_hash,
                            bytes,
                        },
                    )
                })
                .collect(),
            ..Default::default()
        }
    }
}

fn unavailable() -> ApiError {
    ApiError::new(
        "RESOURCE_NOT_READY",
        "Exact job resource bytes are unavailable",
    )
}
