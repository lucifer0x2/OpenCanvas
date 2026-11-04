use crate::{error::Result, node_rows, AppState};
use axum::{
    body::{Body, Bytes},
    extract::{rejection::QueryRejection, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::Response,
};
use moka::sync::Cache;
use opencanvas_storage::{PrivacyStatus, Snapshot, SnapshotCacheIdentity, Storage};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Duration;

#[cfg(test)]
mod tests;
mod validators;

const MAX_CACHE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ENTRY_BYTES: u32 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Format {
    Objects,
    Rows,
}

#[derive(Deserialize)]
pub(crate) struct SnapshotQuery {
    #[serde(rename = "nodeFormat")]
    node_format: Option<String>,
}

#[derive(Clone)]
struct Representation {
    bytes: Bytes,
    etag: HeaderValue,
}

type Key = (SnapshotCacheIdentity, Format, bool);

#[derive(Clone)]
pub(crate) struct SnapshotRepresentations(Cache<Key, Representation>);

impl SnapshotRepresentations {
    pub(crate) fn new() -> Self {
        Self(
            Cache::builder()
                .max_capacity(MAX_CACHE_BYTES)
                .time_to_live(Duration::from_secs(3600))
                .weigher(weight)
                .build(),
        )
    }

    fn read(
        &self,
        store: &Storage,
        project: &str,
        format: Format,
        privacy: PrivacyStatus,
    ) -> opencanvas_storage::Result<Representation> {
        let public = privacy == PrivacyStatus::Public;
        if let Some(identity) = store.snapshot_cache_identity(project)? {
            if let Some(hit) = self.0.get(&(identity, format, public)) {
                return Ok(hit);
            }
        }

        // The source may change between the probe and the snapshot read. Admit
        // the response only under the identity observed by the snapshot read.
        let (snapshot, identity) = store.snapshot_with_cache_identity(project)?;
        let representation = Representation::encode(&snapshot, format, privacy)?;
        if let Some(identity) = identity {
            let key = (identity, format, public);
            if weight(&key, &representation) <= MAX_ENTRY_BYTES {
                return Ok(self.0.get_with(key, || representation));
            }
        }
        Ok(representation)
    }
}

fn weight((identity, _, _): &Key, value: &Representation) -> u32 {
    let bytes = value.bytes.len()
        + value.etag.len()
        + identity.incarnation.len()
        + identity.project_id.len()
        + identity.asset_readiness.len()
        + 128;
    u32::try_from(bytes).unwrap_or(u32::MAX)
}

impl Representation {
    fn encode(
        snapshot: &Snapshot,
        format: Format,
        privacy: PrivacyStatus,
    ) -> serde_json::Result<Self> {
        let mut value = serde_json::to_value(snapshot)?;
        value["truncated"] = json!(false);
        value["privacy_status"] = json!(privacy);
        let tag_prefix = match format {
            Format::Objects => "objects",
            Format::Rows => {
                let nodes = value["nodes"]
                    .as_array_mut()
                    .expect("serialized snapshot nodes must be an array");
                for node in nodes {
                    *node = node_rows::pack(node.take());
                }
                value["nodeFormat"] = json!(node_rows::FORMAT);
                value["nodeColumns"] = json!(node_rows::COLUMNS);
                node_rows::FORMAT
            }
        };
        let bytes = Bytes::from(serde_json::to_vec(&value)?);
        let digest = Sha256::digest(&bytes);
        let etag = HeaderValue::from_str(&format!("\"{tag_prefix}-{digest:x}\""))
            .expect("representation ETag uses only ASCII");
        Ok(Self { bytes, etag })
    }

    fn response(self, request_headers: &HeaderMap) -> Response {
        let mut response = if validators::matches(request_headers, &self.etag) {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::NOT_MODIFIED;
            // Axum would derive a length of zero from the empty body. On 304 a
            // Content-Length, if sent, describes the selected representation.
            response.headers_mut().insert(
                header::CONTENT_LENGTH,
                HeaderValue::from(self.bytes.len() as u64),
            );
            response
        } else {
            let mut response = Response::new(Body::from(self.bytes));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            response
        };
        response.headers_mut().insert(header::ETAG, self.etag);
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-cache"),
        );
        response
    }
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Path(project): Path<String>,
    query: std::result::Result<Query<SnapshotQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response> {
    let format = match query.ok().and_then(|q| q.0.node_format).as_deref() {
        Some("rows") => Format::Rows,
        _ => Format::Objects,
    };
    let cache = state.snapshot_representations.clone();
    let representation = state
        .db(move |store| {
            let meta = store.project_meta(&project)?;
            cache.read(&store, &meta.project_id, format, meta.privacy_status)
        })
        .await?;
    Ok(representation.response(&headers))
}
