use super::{config, http::Proof};
use crate::AppState;
use axum::{body::Body, http::header, response::Response};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::time::{self, Instant, MissedTickBehavior};

const CHECK_EVERY: Duration = Duration::from_secs(3);

/// Keep regular viewer event/replay behavior. End the stream when the admission is no longer
/// valid, including a previously anonymous stream when password protection is turned on.
pub(crate) fn monitor(state: AppState, proof: Option<Proof>, response: Response) -> Response {
    if response
        .headers()
        .get(header::CONTENT_TYPE)
        .is_none_or(|v| v != "text/event-stream")
    {
        return response;
    }
    let (parts, body) = response.into_parts();
    let stream = async_stream::stream! {
        let mut data = body.into_data_stream();
        let mut interval = time::interval_at(Instant::now() + CHECK_EVERY, CHECK_EVERY);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                biased;
                _ = state.shutting_down() => return,
                _ = interval.tick() => {
                    let valid = time::timeout(CHECK_EVERY, async {
                        match &proof {
                            Some(proof) => proof.check(&state).await.unwrap_or(false),
                            None => config(&state).await.is_ok_and(|value| value.is_none()),
                        }
                    }).await.unwrap_or(false);
                    if !valid { return; }
                }
                chunk = data.next() => match chunk {
                    Some(chunk) => yield chunk,
                    None => return,
                }
            }
        }
    };
    Response::from_parts(parts, Body::from_stream(stream))
}
