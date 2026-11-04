//! Admission for every project-scoped public route, including previously issued job URLs.
use crate::{
    error::{ApiError, Result},
    AppState,
};
use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use opencanvas_storage::{PrivacyStatus, ProjectMeta};

enum Scope {
    Project(String),
    Job(String),
}

impl Scope {
    fn for_path(path: &str) -> Option<Self> {
        let (collection, tail) = path.strip_prefix("/api/v1/")?.split_once('/')?;
        let key = tail.split('/').next()?.to_owned();
        if key.is_empty() {
            return None;
        }
        match collection {
            "projects" => Some(Self::Project(key)),
            "jobs" => Some(Self::Job(key)),
            _ => None,
        }
    }

    async fn check(&self, state: &AppState) -> Result<String> {
        match self {
            Self::Project(key) => Ok(public_project(state, key).await?.project_id),
            Self::Job(id) => {
                let owner = state.jobs.project_id(id).await?;
                match public_project(state, &owner).await {
                    Ok(meta) if meta.project_id == owner => Ok(owner),
                    Err(error) if error.0.code != "PROJECT_NOT_FOUND" => Err(error),
                    _ => Err(ApiError::new("JOB_NOT_FOUND", "Render job is unavailable")),
                }
            }
        }
    }

    fn not_found(&self) -> ApiError {
        match self {
            Self::Project(_) => project_not_found(),
            Self::Job(_) => ApiError::new("JOB_NOT_FOUND", "Render job is unavailable"),
        }
    }
}

fn project_not_found() -> ApiError {
    ApiError::new("PROJECT_NOT_FOUND", "Project does not exist")
}

async fn public_project(state: &AppState, key: &str) -> Result<ProjectMeta> {
    match state.project_meta(key.to_owned()).await {
        Ok(meta) if meta.privacy_status == PrivacyStatus::Public => Ok(meta),
        Err(error) if error.0.code != "PROJECT_NOT_FOUND" => Err(error),
        _ => Err(project_not_found()),
    }
}

pub(crate) async fn is_public(state: &AppState, id: &str) -> bool {
    public_project(state, id)
        .await
        .is_ok_and(|meta| meta.project_id == id)
}

pub(crate) async fn run(state: &AppState, request: Request, next: Next) -> Response {
    let Some(scope) = Scope::for_path(request.uri().path()) else {
        return next.run(request).await;
    };
    let admitted = match scope.check(state).await {
        Ok(id) => id,
        Err(error) => return error.into_response(),
    };
    let response = next.run(request).await;
    // Rendering and exports may wait for a worker. Check current ownership before releasing
    // the response, including conditional snapshots and any previously issued downloads.
    match scope.check(state).await {
        Ok(id) if id == admitted => response,
        Ok(_) => scope.not_found().into_response(),
        Err(error) => error.into_response(),
    }
}
