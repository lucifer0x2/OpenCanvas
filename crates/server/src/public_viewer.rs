//! Independent, opt-in public surface. Headers can never grant local mutation privileges.
use crate::{http, viewer_origin::ViewerOrigin, AppState};
use axum::{
    extract::Request,
    http::{header, HeaderValue, Method, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Extension, Json, Router,
};
use serde_json::json;

#[derive(Clone, Debug)]
pub struct PublicViewer {
    origin: String,
    authority: String,
}

impl PublicViewer {
    /// An HTTPS origin only: no credentials, path, query or fragment.
    pub fn new(origin: &str) -> Result<Self, &'static str> {
        let uri: Uri = origin.parse().map_err(|_| "Invalid public viewer origin")?;
        let authority = uri.authority().ok_or("Public viewer origin needs a host")?;
        if uri.scheme_str() != Some("https")
            || authority.as_str().contains('@')
            || origin.contains(['?', '#'])
            || !matches!(uri.path(), "" | "/")
        {
            return Err("Public viewer origin must be an HTTPS origin without a path");
        }
        Ok(Self {
            origin: format!("https://{authority}"),
            authority: authority.to_string(),
        })
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn router(&self, state: AppState) -> Router {
        http::viewer_routes(&state)
            .layer(Extension(ViewerOrigin(self.origin.clone())))
            .layer(middleware::from_fn_with_state(self.clone(), guard))
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
    axum::extract::State(public): axum::extract::State<PublicViewer>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
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
        || (*request.method() == Method::POST && export);
    if !canonical || mcp || !allowed_method {
        return denied(
            "PUBLIC_READ_ONLY",
            "Public viewer permits reads and export requests only",
        );
    }
    let headers = request.headers();
    if headers.get(header::HOST).and_then(|v| v.to_str().ok()) != Some(&public.authority) {
        return denied("HOST_REJECTED", "Configured public Host required");
    }
    if headers
        .get(header::ORIGIN)
        .is_some_and(|v| v != public.origin.as_str())
    {
        return denied("ORIGIN_REJECTED", "Origin is not authorized");
    }
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}
