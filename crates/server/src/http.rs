use crate::{
    error::{ApiError, Result},
    events,
    jobs::SceneRequest,
    mcp::{self, Mcp},
    state::AppState,
    workspace_events,
};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

pub fn router(state: AppState) -> Router {
    let mcp_state = state.clone();
    let config = StreamableHttpServerConfig::default()
        .with_max_request_body_bytes(15 * 1024 * 1024)
        .with_allowed_hosts(["127.0.0.1", "localhost"])
        .with_allowed_origins([state.config.base_url.clone()]);
    let service = StreamableHttpService::new(
        move || Ok(Mcp(mcp_state.clone())),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    let dist = state.config.workspace.join("web/dist");
    Router::new()
        .route("/api/v1/projects", get(projects).post(create))
        .route("/api/v1/projects/{project}/snapshot", get(snapshot))
        .route("/api/v1/events", get(workspace_events::events))
        .route("/api/v1/projects/{project}/events", get(events::events))
        .route("/api/v1/projects/{project}/scene", get(scene))
        .route("/api/v1/projects/{project}/assets/{asset}", get(asset))
        .route("/api/v1/fonts/font_bundled", get(font))
        .route("/api/v1/jobs/{job}", get(job))
        .route("/api/v1/jobs/{job}/image", get(image))
        .nest_service("/mcp", service)
        .fallback_service(
            ServeDir::new(&dist).not_found_service(ServeFile::new(dist.join("index.html"))),
        )
        .layer(DefaultBodyLimit::max(15 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}
async fn projects(
    State(s): State<AppState>,
    query: std::result::Result<Query<mcp::List>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Value>> {
    let q = query
        .map_err(|_| ApiError::invalid("Invalid project query"))?
        .0;
    Ok(Json(
        mcp::projects(
            &s,
            serde_json::to_value(json!({"cursor":q.cursor,"limit":q.limit}))?,
        )
        .await?,
    ))
}
async fn create(
    State(s): State<AppState>,
    body: std::result::Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>> {
    Ok(Json(
        mcp::create(
            &s,
            body.map_err(|_| ApiError::invalid("Invalid JSON body"))?.0,
        )
        .await?,
    ))
}
async fn snapshot(State(s): State<AppState>, Path(id): Path<String>) -> Result<Json<Value>> {
    let mut value = serde_json::to_value(s.snapshot(id).await?)?;
    value["truncated"] = json!(false);
    Ok(Json(value))
}
async fn scene(
    State(s): State<AppState>,
    Path(id): Path<String>,
    query: std::result::Result<Query<SceneRequest>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Value>> {
    Ok(Json(
        mcp::scene(
            &s,
            id,
            query
                .map_err(|_| ApiError::invalid("Invalid scene query"))?
                .0,
        )
        .await?,
    ))
}
async fn job(State(s): State<AppState>, Path(id): Path<String>) -> Result<Json<Value>> {
    Ok(Json(s.jobs.get(&id).await?))
}
async fn image(State(s): State<AppState>, Path(id): Path<String>) -> Result<Response> {
    Ok(binary("image/png", s.jobs.image(&id).await?))
}
async fn asset(
    State(s): State<AppState>,
    Path((project, asset)): Path<(String, String)>,
) -> Result<Response> {
    let (meta, bytes) = s
        .db(move |s| s.open_project(&project)?.read_asset(&asset))
        .await?;
    Ok(binary(&meta.media_type, bytes))
}
async fn font(State(s): State<AppState>) -> Result<Response> {
    let bytes = tokio::fs::read(s.config.workspace.join("renderer/fonts/NotoSans.ttf"))
        .await
        .map_err(|_| ApiError::new("FONT_MISSING", "Bundled font is unavailable"))?;
    Ok(binary("font/ttf", bytes))
}
fn binary(mime: &str, bytes: Vec<u8>) -> Response {
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime).unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    response
}
async fn guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let headers = request.headers();
    let host = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let host_ok = host == "localhost"
        || host == "127.0.0.1"
        || host
            .strip_prefix("localhost:")
            .is_some_and(|p| p.parse::<u16>().is_ok())
        || host
            .strip_prefix("127.0.0.1:")
            .is_some_and(|p| p.parse::<u16>().is_ok());
    if !host_ok {
        return (StatusCode::FORBIDDEN,Json(json!({"error":{"code":"HOST_REJECTED","message":"Loopback Host required","retryable":false}}))).into_response();
    }
    let origin = headers.get(header::ORIGIN).and_then(|h| h.to_str().ok());
    let resource = request.uri().path().contains("/assets/")
        || request.uri().path() == "/api/v1/fonts/font_bundled";
    // Only read-only worker resources permit a random loopback origin. Mutations require same origin.
    let worker_origin = origin.is_some_and(|o| {
        o.strip_prefix("http://127.0.0.1:")
            .is_some_and(|p| p.parse::<u16>().is_ok())
    });
    if origin.is_some_and(|o| o != state.config.base_url) && !(resource && worker_origin) {
        return (StatusCode::FORBIDDEN,Json(json!({"error":{"code":"ORIGIN_REJECTED","message":"Origin is not authorized","retryable":false}}))).into_response();
    }
    let origin = origin.map(String::from);
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    if resource {
        if let Some(origin) = origin {
            if let Ok(value) = HeaderValue::from_str(&origin) {
                response
                    .headers_mut()
                    .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
                response
                    .headers_mut()
                    .insert(header::VARY, HeaderValue::from_static("Origin"));
            }
        }
    }
    response
}
