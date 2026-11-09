//! Independent, opt-in public surface. Headers can never grant local mutation privileges.
use crate::{
    http,
    viewer_auth::{self, http as auth, http::Access},
    viewer_privacy, AppState,
};
use axum::{
    extract::{DefaultBodyLimit, Request, State},
    http::{header, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Extension, Json, Router,
};
use serde_json::json;
use std::net::IpAddr;

mod origins;

#[derive(Clone, Debug)]
pub struct PublicViewer {
    origins: origins::AllowedOrigins,
    pub(crate) trusted_proxy: Option<IpAddr>,
}

impl PublicViewer {
    /// An HTTPS origin only: no credentials, path, query or fragment.
    pub fn new(origin: &str) -> Result<Self, &'static str> {
        Ok(Self {
            origins: origins::AllowedOrigins::new(origin)?,
            trusted_proxy: None,
        })
    }

    pub fn origin(&self) -> &str {
        self.origins.primary()
    }

    /// Add a comma-separated list of exact HTTPS origins (at most eight total).
    pub fn with_allowed_origins(mut self, origins: &str) -> Result<Self, &'static str> {
        self.origins = self.origins.additional(origins)?;
        Ok(self)
    }

    /// Accept one client address from X-Forwarded-For only from this transport peer.
    pub fn with_trusted_proxy(mut self, proxy: IpAddr) -> Self {
        self.trusted_proxy = Some(proxy.to_canonical());
        self
    }

    pub fn router(&self, state: AppState) -> Router {
        http::viewer_routes(&state)
            .route(
                viewer_auth::PATH,
                get(auth::public_status)
                    .post(auth::login)
                    .layer(DefaultBodyLimit::max(16 * 1024)),
            )
            .layer(Extension(self.clone()))
            .layer(middleware::from_fn_with_state(
                (self.clone(), state.clone()),
                guard,
            ))
            .with_state(state)
    }
}

fn denied(code: &str, message: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({"error":{
            "code":code,"message":message,"retryable":false
        }})),
    )
        .into_response()
}

async fn guard(
    State((public, state)): State<(PublicViewer, AppState)>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    let auth_path = path == viewer_auth::PATH;
    let login = auth_path && request.method() == Method::POST;
    // Reject ambiguous encodings instead of interpreting a different path from the router.
    // Public project/font/asset IDs and built asset filenames use unescaped ASCII paths.
    let parts: Vec<_> = path.split('/').collect();
    let canonical = !path.contains(['%', '\\'])
        && !path.contains("//")
        && !parts.iter().any(|p| matches!(*p, "." | ".."));
    let mcp = parts.get(1).is_some_and(|p| p.eq_ignore_ascii_case("mcp"));
    let export = parts.len() == 6
        && parts[1..4] == ["api", "v1", "projects"]
        && !parts[4].is_empty()
        && parts[5] == "exports";
    let allowed_method = matches!(*request.method(), Method::GET | Method::HEAD)
        || login
        || (*request.method() == Method::POST && export);
    if !canonical || mcp || !allowed_method {
        return denied(
            "PUBLIC_READ_ONLY",
            "Public viewer permits reads, exports and viewer sign-in only",
        );
    }
    let origin = match public.origins.request(request.headers(), login) {
        Ok(origin) => origin,
        Err(code) => return denied(code, "Configured public Host and matching Origin required"),
    };
    // Public data routes use /api/. The shell and the capability/status probes must load before login.
    let api = path.starts_with("/api/");
    let protected = api && !auth_path && path != "/api/v1/viewer";
    request.extensions_mut().insert(origin.clone());
    let mut response = if protected {
        match auth::access(&state, &origin.0, auth::presented(&public, &request)).await {
            Ok(Access::Open) => {
                let response = viewer_privacy::run(&state, request, next).await;
                viewer_auth::stream::monitor(state, None, response)
            }
            Ok(Access::Session(proof)) => {
                let response = viewer_privacy::run(&state, request, next).await;
                viewer_auth::stream::monitor(state, Some(proof), response)
            }
            Ok(Access::Locked) => auth::unauthorized(),
            Err(error) => error.into_response(),
        }
    } else {
        next.run(request).await
    };
    if api {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}
