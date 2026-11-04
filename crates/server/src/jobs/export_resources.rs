//! Store-backed Chromium exports and immutable native job resources share SVG embedding.
use super::resource_bytes::ResourceBytes;
use crate::error::{ApiError, Result};
use opencanvas_storage::{Asset, Font, Storage};
use std::{sync::Arc, time::Instant};

#[derive(Clone)]
pub(super) struct AssetSource {
    pub store: Storage,
    pub project_id: String,
    pub cached: Option<Arc<ResourceBytes>>,
    pub deadline: Option<Instant>,
}

impl AssetSource {
    pub async fn blocking<T: Send + 'static>(
        &self,
        run: impl FnOnce() -> Result<T> + Send + 'static,
    ) -> Result<T> {
        if let Some(deadline) = self.deadline {
            return super::native::blocking(deadline, move |control| {
                control.check()?;
                let result = run();
                control.check()?;
                result
            })
            .await;
        }
        tokio::task::spawn_blocking(run)
            .await
            .map_err(|_| ApiError::new("INTERNAL_ERROR", "Export resource task failed"))?
    }
    pub fn read_asset(&self, id: &str) -> Result<(Asset, Vec<u8>)> {
        match &self.cached {
            Some(bytes) => bytes.asset(id),
            None => self
                .store
                .open_project(&self.project_id)
                .and_then(|p| p.read_asset(id))
                .map_err(|_| unavailable()),
        }
    }
    pub fn read_font(&self, id: &str) -> Result<(Font, String, Vec<u8>)> {
        match &self.cached {
            Some(bytes) => bytes.font(id),
            None => self
                .store
                .open_project(&self.project_id)
                .and_then(|p| p.read_font(id))
                .map_err(|_| unavailable()),
        }
    }
}
fn unavailable() -> ApiError {
    ApiError::new(
        "RESOURCE_NOT_READY",
        "Exact project resource bytes are unavailable",
    )
}
