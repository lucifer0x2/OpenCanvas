use crate::{
    error::{ApiError, Result},
    state::AppState,
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
use std::{convert::Infallible, time::Duration};

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    cursor: Option<String>,
}
pub async fn events(
    State(state): State<AppState>,
    Path(project): Path<String>,
    query: std::result::Result<Query<Cursor>, axum::extract::rejection::QueryRejection>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let query = query
        .map_err(|_| ApiError::invalid("Invalid events query"))?
        .0;
    // Subscription exists before snapshot and durable log reads. No DB handle crosses a yield.
    let mut receiver = state.events.subscribe();
    let snapshot = state.snapshot(project.clone()).await?;
    let project = snapshot.project_id;
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
    let invalid =
        raw.is_some() && parsed.is_none_or(|(id, r)| id != project || r > snapshot.revision);
    let mut cursor = parsed.map(|(_, r)| r).unwrap_or(snapshot.revision);
    let stream = async_stream::stream! {
        if invalid {
            yield Ok::<Event,Infallible>(reset(&project,snapshot.revision,"invalid_cursor"));return;
        }
        loop {
            let id=project.clone();let from=cursor;
            let replay=state.db(move|s|s.open_project(&id)?.commits_after(from,1000)).await;
            match replay {
                Ok(commits)=> {
                    let full=commits.len()==1000;
                    for commit in commits {
                        if commit.revision != cursor+1 {
                            let current=state.snapshot(project.clone()).await.map(|s|s.revision).unwrap_or(cursor);
                            yield Ok(reset(&project,current,"replay_gap"));return;
                        }
                        cursor=commit.revision;
                        let data=json!({"projectId":project,"revision":cursor,"changedIds":commit.changed_ids});
                        yield Ok(event("commit",&project,cursor,data));
                    }
                    if full {continue;}
                    let current=state.snapshot(project.clone()).await.map(|s|s.revision).unwrap_or(cursor);
                    if current>cursor {
                        // A commit can race the log read; read once more before concluding a gap.
                        let id=project.clone();let from=cursor;
                        match state.db(move|s|s.open_project(&id)?.commits_after(from,1)).await {
                            Ok(next) if next.first().is_some_and(|c|c.revision==cursor+1)=>continue,
                            _=>{yield Ok(reset(&project,current,"replay_gap"));return;}
                        }
                    }
                }
                Err(_)=>{
                    let current=state.snapshot(project.clone()).await.map(|s|s.revision).unwrap_or(cursor);
                    yield Ok(reset(&project,current,"replay_unavailable"));return;
                }
            }
            // Broadcast is a wakeup channel; commits always come from the durable ordered log.
            // A stopping server must not wait for this stream's client to disconnect first.
            let next = tokio::select! {
                biased;
                _ = state.shutting_down() => return,
                message = receiver.recv() => message,
            };
            match next {
                Ok(message)=> {
                    if message["projectId"]!=project {continue;}
                    if message["event"]=="scene_ready" {
                        yield Ok(Event::default().event("scene_ready").data(message.to_string()));
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_))=> {
                    let current=state.snapshot(project.clone()).await.map(|s|s.revision).unwrap_or(cursor);
                    yield Ok(reset(&project,current,"buffer_overflow"));return;
                }
                Err(_)=>return,
            }
        }
    };
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("heartbeat"),
    ))
}
fn event(kind: &str, project: &str, revision: u64, data: Value) -> Event {
    Event::default()
        .event(kind)
        .id(format!("{project}:{revision}"))
        .data(data.to_string())
}
fn reset(project: &str, revision: u64, reason: &str) -> Event {
    Event::default()
        .event("reset_required")
        .data(json!({"projectId":project,"currentRevision":revision,"reason":reason}).to_string())
}
