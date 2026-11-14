//! Prepare immutable render work before taking the queue lock.
use super::{input, ExportSpec, SceneRequest};
use crate::{error::Result, state::Config};
use opencanvas_storage::Snapshot;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub(super) struct Prepared {
    pub key: String,
    pub input: Value,
    pub overflow: Vec<String>,
    pub project: String,
    pub revision: u64,
    pub request: SceneRequest,
    pub export: Option<ExportSpec>,
}

impl Prepared {
    pub fn new(
        snapshot: &Snapshot,
        request: SceneRequest,
        config: &Config,
        export: Option<ExportSpec>,
    ) -> Result<Self> {
        input::check_request(snapshot, &request)?;
        let input = input::make_input(snapshot, &request, &config.base_url)?;
        let overflow = if snapshot.notes.is_empty() {
            Vec::new()
        } else {
            crate::text_diagnostics::overflow_ids(
                snapshot,
                &serde_json::from_value::<Vec<opencanvas_storage::Node>>(input["nodes"].clone())?,
            )
        };
        Self::from_input(snapshot, request, config, export, input, overflow)
    }

    pub(super) fn from_input(
        snapshot: &Snapshot,
        request: SceneRequest,
        config: &Config,
        export: Option<ExportSpec>,
        input: Value,
        overflow: Vec<String>,
    ) -> Result<Self> {
        let mut basis =
            json!({"input":input,"scale":request.scale,"pipeline":config.pipeline_identity});
        if let Some(spec) = &export {
            basis["export"] = json!(spec);
        }
        let key = format!("{:x}", Sha256::digest(serde_json::to_vec(&basis)?));
        Ok(Self {
            key,
            input,
            overflow,
            project: snapshot.project_id.clone(),
            revision: snapshot.revision,
            request,
            export,
        })
    }
}
