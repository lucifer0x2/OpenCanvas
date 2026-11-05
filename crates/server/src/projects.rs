use crate::{
    error::{ApiError, Result},
    state::AppState,
    viewer_origin::Origin,
    workspace_events,
};
use axum::{
    extract::{rejection::JsonRejection, rejection::QueryRejection, Path, Query, State},
    Json,
};
use opencanvas_storage::{PrivacyStatus, ProjectMeta};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Create {
    slug: String,
    name: String,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct List {
    cursor: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Privacy {
    privacy_status: PrivacyStatus,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Update {
    #[serde(rename = "projectId")]
    project_id: String,
    privacy_status: PrivacyStatus,
}

pub(crate) async fn list(state: &AppState, query: List, public_only: bool) -> Result<Value> {
    let limit = query.limit.unwrap_or(100);
    if !(1..=100).contains(&limit) {
        return Err(ApiError::invalid("Project limit must be 1–100"));
    }
    let all = state.db(|store| store.list_projects()).await?;
    let mut page = all
        .into_iter()
        .filter(|meta| !public_only || meta.privacy_status == PrivacyStatus::Public)
        .filter(|meta| {
            query
                .cursor
                .as_ref()
                .is_none_or(|cursor| meta.slug > *cursor)
        })
        .take(limit + 1)
        .collect::<Vec<_>>();
    let next = if page.len() > limit {
        page.pop();
        page.last().map(|meta| &meta.slug)
    } else {
        None
    };
    Ok(json!({"projects":page,"nextCursor":next}))
}

pub(crate) async fn create(state: &AppState, request: Create) -> Result<ProjectMeta> {
    let meta = state
        .db(move |store| {
            let project = store.create_project(&request.slug, &request.name)?;
            store.project_meta(project.id())
        })
        .await?;
    let _ = state.events.send(workspace_events::project_created(&meta));
    Ok(meta)
}

pub(crate) async fn update(state: &AppState, request: Update) -> Result<ProjectMeta> {
    let (meta, changed) = state
        .db(move |store| store.update_project_privacy(&request.project_id, request.privacy_status))
        .await?;
    if changed {
        let _ = state.events.send(json!({
            "event":"project_visibility_changed",
            "projectId":meta.project_id,
            "privacy_status":meta.privacy_status,
        }));
    }
    Ok(meta)
}

pub(crate) async fn list_http(
    State(state): State<AppState>,
    origin: Origin,
    query: std::result::Result<Query<List>, QueryRejection>,
) -> Result<Json<Value>> {
    let Query(query) = query.map_err(|_| ApiError::invalid("Invalid project query"))?;
    Ok(Json(list(&state, query, origin.is_some()).await?))
}

pub(crate) async fn create_http(
    State(state): State<AppState>,
    body: std::result::Result<Json<Create>, JsonRejection>,
) -> Result<Json<ProjectMeta>> {
    let Json(request) = body.map_err(|_| ApiError::invalid("Invalid project body"))?;
    Ok(Json(create(&state, request).await?))
}

pub(crate) async fn get_http(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Result<Json<ProjectMeta>> {
    Ok(Json(state.project_meta(project).await?))
}

pub(crate) async fn update_http(
    State(state): State<AppState>,
    Path(project): Path<String>,
    body: std::result::Result<Json<Privacy>, JsonRejection>,
) -> Result<Json<ProjectMeta>> {
    let Json(body) = body.map_err(|_| ApiError::invalid("Invalid project privacy body"))?;
    Ok(Json(
        update(
            &state,
            Update {
                project_id: project,
                privacy_status: body.privacy_status,
            },
        )
        .await?,
    ))
}
