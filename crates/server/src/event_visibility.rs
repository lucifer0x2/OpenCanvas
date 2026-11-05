//! SSE scope checks run at delivery time so queued hints and durable replay stay private.
use crate::{viewer_privacy::is_public, AppState};
use axum::response::sse::Event;
use futures_util::{Stream, StreamExt};
use opencanvas_storage::PrivacyStatus;
use std::{collections::HashSet, convert::Infallible, time::Duration};
use tokio::time::{self, Instant, MissedTickBehavior};

const CHECK_EVERY: Duration = Duration::from_secs(3);

pub(crate) enum LiveEvent {
    Scoped(Event, String),
    Workspace(Event),
    Created(Event),
    Visibility { project: String, public: bool },
}

impl LiveEvent {
    fn local(self) -> Event {
        match self {
            Self::Scoped(event, _) | Self::Workspace(event) | Self::Created(event) => event,
            Self::Visibility { .. } => visibility_changed(),
        }
    }
}

fn visibility_changed() -> Event {
    Event::default()
        .event("project_visibility_changed")
        .data("{}")
}

pub(crate) async fn public_projects(state: &AppState) -> crate::error::Result<HashSet<String>> {
    state
        .db(|store| {
            Ok(store
                .list_projects()?
                .into_iter()
                .filter(|meta| meta.privacy_status == PrivacyStatus::Public)
                .map(|meta| meta.project_id)
                .collect())
        })
        .await
}

pub(crate) fn stream<S: Stream<Item = LiveEvent>>(
    state: AppState,
    source: S,
    owner: Option<String>,
    public: Option<HashSet<String>>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    async_stream::stream! {
        let mut source = std::pin::pin!(source);
        let Some(mut projects) = public else {
            while let Some(event) = source.next().await {
                yield Ok(event.local());
            }
            return;
        };
        let mut interval = time::interval_at(Instant::now() + CHECK_EVERY, CHECK_EVERY);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                biased;
                _ = state.shutting_down() => return,
                _ = interval.tick() => {
                    let current = public_projects(&state).await;
                    if let Some(id) = &owner {
                        if !is_public(&state, id).await {
                            yield Ok(visibility_changed());
                            return;
                        }
                    }
                    if let Ok(current) = current {
                        if projects != current {
                            projects = current;
                            yield Ok(visibility_changed());
                        }
                    }
                }
                next = source.next() => {
                    let Some(event) = next else { return };
                    let visible = match &event {
                        LiveEvent::Scoped(_, project) if owner.as_ref() != Some(project) => {
                            is_public(&state, project).await
                        }
                        LiveEvent::Created(_) => false,
                        _ => true,
                    };
                    if let Some(id) = &owner {
                        if matches!(&event, LiveEvent::Visibility { project, public: false } if project == id)
                            || !is_public(&state, id).await
                        {
                            yield Ok(visibility_changed());
                            return;
                        }
                    }
                    if !visible {
                        continue;
                    }
                    match event {
                        LiveEvent::Scoped(event, _) | LiveEvent::Workspace(event) => yield Ok(event),
                        // New projects are always private. Publication sends a visibility hint.
                        LiveEvent::Created(_) => {}
                        LiveEvent::Visibility { project, public } => {
                            if public { projects.insert(project); } else { projects.remove(&project); }
                            yield Ok(visibility_changed());
                        }
                    }
                }
            }
        }
    }
}
