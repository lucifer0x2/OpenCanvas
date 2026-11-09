//! Cache policy belongs to static files only, shared by both viewer listeners.
use axum::{
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::Response,
    Router,
};
use std::path::Path;
use tower_http::services::{ServeDir, ServeFile};

const NO_STORE: &str = "no-store";
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

pub(crate) fn service(dist: &Path) -> Router {
    // A missing hash-looking URL must not validate against index.html's mtime.
    let shell = Router::new()
        .fallback_service(ServeFile::new(dist.join("index.html")))
        .layer(middleware::from_fn(shell_policy));
    Router::new()
        .fallback_service(
            ServeDir::new(dist)
                .precompressed_br()
                .precompressed_gzip()
                .not_found_service(shell),
        )
        .layer(middleware::from_fn(cache_policy))
}

async fn shell_policy(mut request: Request, next: Next) -> Response {
    discard_validators(&mut request);
    let mut response = next.run(request).await;
    no_store(&mut response);
    response
}

async fn cache_policy(mut request: Request, next: Next) -> Response {
    let hashed = hashed_bundle(request.uri().path());
    if !hashed {
        discard_validators(&mut request);
    }
    let mut response = next.run(request).await;
    let headers = response.headers();
    let bundle_type = headers.get(header::CONTENT_TYPE).is_some_and(|value| {
        matches!(
            value.to_str().unwrap_or("").split(';').next(),
            Some("text/javascript" | "application/javascript" | "text/css")
        )
    });
    let immutable = hashed
        && headers.get(header::CACHE_CONTROL) != Some(&HeaderValue::from_static(NO_STORE))
        && (response.status() == StatusCode::NOT_MODIFIED
            || (response.status().is_success() && bundle_type));
    if immutable {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static(IMMUTABLE));
    } else {
        no_store(&mut response);
    }
    // Keep representation negotiation even when a conditional response has no body.
    response
        .headers_mut()
        .append(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    response
}

fn discard_validators(request: &mut Request) {
    // Release archives may preserve mtimes. Never let them validate an old shell
    // or stable-name WASM/JS, including HEAD, ranges and wildcard entity tags.
    for name in [
        header::IF_MODIFIED_SINCE,
        header::IF_UNMODIFIED_SINCE,
        header::IF_NONE_MATCH,
        header::IF_MATCH,
        header::IF_RANGE,
        header::RANGE,
    ] {
        request.headers_mut().remove(name);
    }
}

fn no_store(response: &mut Response) {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(NO_STORE));
    headers.remove(header::LAST_MODIFIED);
    headers.remove(header::ETAG);
}

fn hashed_bundle(path: &str) -> bool {
    // Vite's default direct-child assets/<name>-<8 base64url hash>.js|css.
    // No other extension, directory, encoding or stable scene-wasm URL qualifies.
    let Some(file) = path.strip_prefix("/assets/") else {
        return false;
    };
    let Some(stem) = file
        .strip_suffix(".js")
        .or_else(|| file.strip_suffix(".css"))
    else {
        return false;
    };
    if !stem
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        || stem.len() < 10
    {
        return false;
    }
    stem.as_bytes()[stem.len() - 9] == b'-'
}
