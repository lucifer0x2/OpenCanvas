//! Bounded, revision-consistent page render admission. Clients finish one batch before paging.
use crate::{
    error::{ApiError, Result},
    jobs::SceneRequest,
    viewer_origin::{self, Origin},
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use opencanvas_storage::{Node, NodeKind, Snapshot};
use serde::Deserialize;
use serde_json::{json, Value};

fn batch_limit() -> usize {
    4
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ScenesQuery {
    page_id: String,
    revision: u64,
    #[serde(default)]
    offset: usize,
    #[serde(default = "batch_limit")]
    limit: usize,
    width: Option<f64>,
    height: Option<f64>,
}

impl ScenesQuery {
    fn validate(&self) -> Result<()> {
        if self.page_id.is_empty() || !(1..=4).contains(&self.limit) {
            return Err(ApiError::invalid(
                "pageId is required and limit must be between 1 and 4",
            ));
        }
        match (self.width, self.height) {
            (None, None) => Ok(()),
            (Some(width), Some(height)) => {
                opencanvas_scene_input::check_viewport(width, height, 1.0)?;
                Ok(())
            }
            _ => Err(ApiError::invalid(
                "width and height must be supplied together",
            )),
        }
    }

    fn frames<'a>(&self, snapshot: &'a Snapshot) -> Result<Vec<&'a Node>> {
        if snapshot.revision != self.revision {
            return Err(ApiError::new(
                "REVISION_UNAVAILABLE",
                "Requested authored revision is unavailable",
            ));
        }
        if !snapshot
            .nodes
            .iter()
            .any(|node| node.id == self.page_id && node.kind == NodeKind::Page)
        {
            return Err(ApiError::invalid("pageId must identify a page"));
        }
        let mut frames: Vec<_> = snapshot
            .nodes
            .iter()
            .filter(|node| {
                node.kind == NodeKind::Frame
                    && node.parent_id.as_deref() == Some(self.page_id.as_str())
            })
            .collect();
        frames.sort_by(|a, b| (a.sort_key, &a.id).cmp(&(b.sort_key, &b.id)));
        if self.offset > frames.len() {
            return Err(ApiError::invalid(
                "offset exceeds the number of root frames",
            ));
        }
        Ok(frames)
    }

    fn request(&self, frame: &Node) -> SceneRequest {
        SceneRequest {
            frame_id: frame.id.clone(),
            revision: self.revision,
            width: self
                .width
                .unwrap_or_else(|| dimension(frame, "width", 800.0)),
            height: self
                .height
                .unwrap_or_else(|| dimension(frame, "height", 600.0)),
            theme: "default".into(),
            state: "default".into(),
            scale: 1.0,
        }
    }
}

fn dimension(frame: &Node, name: &str, fallback: f64) -> f64 {
    frame.props[name]
        .as_f64()
        .filter(|n| n.is_finite() && *n > 0.0)
        .unwrap_or(fallback)
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Path(project): Path<String>,
    origin: Origin,
    query: std::result::Result<Query<ScenesQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Value>> {
    let query = query
        .map_err(|_| ApiError::invalid("Invalid scenes query"))?
        .0;
    query.validate()?;
    let snapshot = state.snapshot(project).await?;
    let frames = query.frames(&snapshot)?;
    let total = frames.len();
    let end = query.offset.saturating_add(query.limit).min(total);
    let requests = frames[query.offset..end]
        .iter()
        .map(|frame| query.request(frame))
        .collect();
    let jobs: Vec<_> = state
        .jobs
        .enqueue_batch(&snapshot, requests)
        .await?
        .into_iter()
        .map(|job| viewer_origin::job(job, &state.config.base_url, origin.clone()))
        .collect();
    let next = (end < total).then_some(end);
    Ok(Json(
        json!({"projectId":snapshot.project_id,"revision":snapshot.revision,
        "pageId":query.page_id,"jobs":jobs,"nextOffset":next,"total":total}),
    ))
}

#[cfg(test)]
#[path = "scenes_tests.rs"]
mod tests;
