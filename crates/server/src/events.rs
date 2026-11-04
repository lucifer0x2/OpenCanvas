use crate::{
    error::{ApiError, Result},
    event_visibility::{self, LiveEvent},
    state::AppState,
    viewer_origin::Origin,
    workspace_events::{classify, Signal},
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    cursor: Option<String>,
}
pub async fn events(
    State(state): State<AppState>,
    Path(project): Path<String>,
    origin: Origin,
    query: std::result::Result<Query<Cursor>, axum::extract::rejection::QueryRejection>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let query = query
        .map_err(|_| ApiError::invalid("Invalid events query"))?
        .0;
    // Subscription exists before metadata and durable log reads. No DB handle crosses a yield.
    // Only the canonical ID and revision are needed, so no read here loads the whole design.
    let mut receiver = state.events.subscribe();
    let meta = state.project_meta(project.clone()).await?;
    let (project, opened) = (meta.project_id, meta.revision);
    let public = if origin.is_some() {
        Some(event_visibility::public_projects(&state).await?)
    } else {
        None
    };
    let raw = query.cursor.or_else(|| {
        headers
            .get("last-event-id")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    });
    let parsed = raw
        .as_ref()
        .and_then(|s| s.rsplit_once(':'))
        .and_then(|(id, r)| r.parse::<u64>().ok().map(|r| (id, r)));
    let invalid = raw.is_some() && parsed.is_none_or(|(id, r)| id != project || r > opened);
    let mut cursor = parsed.map(|(_, r)| r).unwrap_or(opened);
    let owner = project.clone();
    let stream_state = state.clone();
    let stream = async_stream::stream! {
        let state = stream_state;
        if invalid {
            yield reset(&project, opened, "invalid_cursor");
            return;
        }
        loop {
            let id=project.clone();let from=cursor;
            let replay=state.db(move|s|s.open_project(&id)?.commits_after(from,1000)).await;
            match replay {
                Ok(commits)=> {
                    let full=commits.len()==1000;
                    for commit in commits {
                        if commit.revision != cursor+1 {
                            let current=state.revision_or(project.clone(),cursor).await;
                            yield reset(&project, current, "replay_gap");
                            return;
                        }
                        cursor=commit.revision;
                        let data=json!({"projectId":project,"revision":cursor,"changedIds":commit.changed_ids});
                        yield commit_event(&project, cursor, data);
                    }
                    if full {continue;}
                    let current=state.revision_or(project.clone(),cursor).await;
                    if current>cursor {
                        // A commit can race the log read; read once more before concluding a gap.
                        let id=project.clone();let from=cursor;
                        match state.db(move|s|s.open_project(&id)?.commits_after(from,1)).await {
                            Ok(next) if next.first().is_some_and(|c|c.revision==cursor+1)=>continue,
                            _ => {
                                yield reset(&project, current, "replay_gap");
                                return;
                            }
                        }
                    }
                }
                Err(_)=>{
                    let current=state.revision_or(project.clone(),cursor).await;
                    yield reset(&project, current, "replay_unavailable");
                    return;
                }
            }
            // Broadcast is a wakeup channel; commits always come from the durable ordered log.
            // Only a commit of this project reopens the log. Public delivery separately checks
            // current visibility for both replayed events and workspace hints.
            // A stopping server must not wait for this stream's client to disconnect first.
            loop {
                let next = tokio::select! {
                    biased;
                    _ = state.shutting_down() => return,
                    message = receiver.recv() => message,
                };
                match next {
                    Ok(message)=> match classify(&message, Some(&project)) {
                        Signal::Replay=>break,
                        Signal::Forward(event)=>yield event,
                        Signal::Ignore=>{}
                    },
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=> {
                        let current=state.revision_or(project.clone(),cursor).await;
                        yield reset(&project, current, "buffer_overflow");
                        return;
                    }
                    Err(_)=>return,
                }
            }
        }
    };
    Ok(
        Sse::new(event_visibility::stream(state, stream, Some(owner), public)).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(10))
                .text("heartbeat"),
        ),
    )
}
fn commit_event(project: &str, revision: u64, data: Value) -> LiveEvent {
    LiveEvent::Scoped(
        Event::default()
            .event("commit")
            .id(format!("{project}:{revision}"))
            .data(data.to_string()),
        project.into(),
    )
}
fn reset(project: &str, revision: u64, reason: &str) -> LiveEvent {
    LiveEvent::Scoped(
        Event::default().event("reset_required").data(
            json!({"projectId":project,"currentRevision":revision,"reason":reason}).to_string(),
        ),
        project.into(),
    )
}
