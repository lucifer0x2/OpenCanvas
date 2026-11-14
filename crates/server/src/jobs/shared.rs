//! Authoritative runtime preparation; custom snapshots retain full validation.
use super::{input, prepared::Prepared, queue::Priority, ExportSpec, Jobs, SceneRequest};
use crate::{error::Result, state::Config};
use opencanvas_scene_input::{scene_diagnostics::Intent, SceneOptions};
use opencanvas_storage::SharedSnapshot;
use serde_json::Value;

#[cfg(test)]
#[path = "shared_tests.rs"]
mod tests;

impl Prepared {
    pub(super) fn shared(
        shared: &SharedSnapshot,
        request: SceneRequest,
        config: &Config,
        export: Option<ExportSpec>,
    ) -> Result<Self> {
        let snapshot = &shared.snapshot;
        input::check_request(snapshot, &request)?;
        let Some(scoped) = shared.trusted_scoped()? else {
            return Self::new(snapshot, request, config, export);
        };
        let options = SceneOptions {
            frame_id: request.frame_id.clone(),
            width: request.width,
            height: request.height,
            theme: request.theme.clone(),
            state: request.state.clone(),
        };
        let (mut input, nodes) = scoped.prepare_input_with_scale(&options, request.scale)?;
        input::attach_urls(&mut input, &snapshot.project_id, &config.base_url);
        let overflow = if snapshot.notes.is_empty() {
            Vec::new()
        } else {
            let notes =
                scoped.notes_for_nodes(nodes.dependencies.iter().map(|node| node.id.as_str()));
            let intent = Intent::with_nodes_and_notes(nodes.dependencies.iter().copied(), notes);
            let rendered =
                serde_json::from_value::<Vec<opencanvas_storage::Node>>(input["nodes"].clone())?;
            intent.overflow_ids(&rendered)
        };
        Self::from_input(snapshot, request, config, export, input, overflow)
    }
}

impl Jobs {
    pub(crate) async fn enqueue_shared(
        &self,
        snapshot: &SharedSnapshot,
        request: SceneRequest,
    ) -> Result<Value> {
        self.admit_shared(snapshot, request, true, Priority::Direct, None)
            .await
    }

    pub(crate) async fn context_shared(
        &self,
        snapshot: &SharedSnapshot,
        request: SceneRequest,
    ) -> Result<Value> {
        self.admit_shared(snapshot, request, false, Priority::Direct, None)
            .await
    }

    pub(crate) async fn prerender_shared(
        &self,
        snapshot: &SharedSnapshot,
        request: SceneRequest,
    ) -> Result<Value> {
        self.admit_shared(snapshot, request, true, Priority::Background, None)
            .await
    }

    pub(crate) async fn export_shared(
        &self,
        snapshot: &SharedSnapshot,
        request: SceneRequest,
        spec: ExportSpec,
    ) -> Result<Value> {
        self.admit_shared(snapshot, request, true, Priority::Direct, Some(spec))
            .await
    }

    async fn admit_shared(
        &self,
        snapshot: &SharedSnapshot,
        request: SceneRequest,
        retry: bool,
        priority: Priority,
        export: Option<ExportSpec>,
    ) -> Result<Value> {
        let prepared = Prepared::shared(snapshot, request, &self.config, export)?;
        self.admit_one(prepared, retry, priority).await
    }
}
