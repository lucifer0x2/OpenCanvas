//! Workspace-level live hints: `project_created` and `project_activity`.
//!
//! Both are non-durable hints carried on the existing broadcast. They have no SSE `id`, so the
//! commit cursor of a project stream keeps its meaning; the list API stays the source of truth.
use crate::{
    error::{ApiError, Result},
    event_visibility::{self, LiveEvent},
    state::AppState,
    viewer_origin::Origin,
};
use axum::{
    extract::{Query, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
};
use opencanvas_storage::ProjectMeta;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;

/// What a stream does with one broadcast message.
pub(crate) enum Signal {
    /// A commit of the stream's own project: read the durable log.
    Replay,
    /// A hint to forward, with scope information for public delivery.
    Forward(LiveEvent),
    Ignore,
}

/// Classifies a broadcast message for the stream of `own` (None for the workspace stream).
pub(crate) fn classify(message: &Value, own: Option<&str>) -> Signal {
    let Some(project) = message["projectId"].as_str() else {
        return Signal::Ignore;
    };
    let mine = own == Some(project);
    match message["event"].as_str() {
        Some("commit") if mine => Signal::Replay,
        Some("commit") => Signal::Forward(LiveEvent::Scoped(
            hint(
                "project_activity",
                json!({"projectId":project,"revision":message["revision"]}),
            ),
            project.into(),
        )),
        Some("project_created") => Signal::Forward(LiveEvent::Created(hint(
            "project_created",
            json!({
                "projectId":message["projectId"],
                "slug":message["slug"],
                "name":message["name"],
                "revision":message["revision"],
            }),
        ))),
        Some("scene_ready") if mine => Signal::Forward(LiveEvent::Scoped(
            Event::default()
                .event("scene_ready")
                .data(message.to_string()),
            project.into(),
        )),
        Some("project_visibility_changed") => Signal::Forward(LiveEvent::Visibility {
            project: project.into(),
            public: message["privacy_status"] == "public",
        }),
        _ => Signal::Ignore,
    }
}

/// The broadcast message `create_project` sends after a successful creation.
pub(crate) fn project_created(created: &ProjectMeta) -> Value {
    json!({
        "event":"project_created",
        "projectId":created.project_id,
        "slug":created.slug,
        "name":created.name,
        "revision":created.revision,
    })
}

fn hint(kind: &str, data: Value) -> Event {
    Event::default().event(kind).data(data.to_string())
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct NoQuery {}

/// `GET /api/v1/events`: workspace hints for a viewer that has no project to follow yet.
/// No cursor and no replay; a client refetches `GET /api/v1/projects` on every (re)open.
pub async fn events(
    State(state): State<AppState>,
    origin: Origin,
    query: std::result::Result<Query<NoQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<impl IntoResponse> {
    query.map_err(|_| ApiError::invalid("Invalid events query"))?;
    let mut receiver = state.events.subscribe();
    let public = if origin.is_some() {
        Some(event_visibility::public_projects(&state).await?)
    } else {
        None
    };
    let stream_state = state.clone();
    let stream = async_stream::stream! {
        loop {
            let next = tokio::select! {
                biased;
                _ = stream_state.shutting_down() => return,
                message = receiver.recv() => message,
            };
            match next {
                Ok(message) => {
                    if let Signal::Forward(event) = classify(&message, None) {
                        yield event;
                    }
                }
                Err(RecvError::Lagged(_)) => {
                    let data = json!({"reason":"buffer_overflow"}).to_string();
                    yield LiveEvent::Workspace(Event::default().event("reset_required").data(data));
                    return;
                }
                Err(RecvError::Closed) => return,
            }
        }
    };
    Ok(
        Sse::new(event_visibility::stream(state, stream, None, public)).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(10))
                .text("heartbeat"),
        ),
    )
}
