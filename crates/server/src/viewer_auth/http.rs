use super::{admin::auth, config, crypto, now, valid_password, LIFETIME};
use crate::{error::Result, ApiError, AppState, PublicViewer};
use axum::{
    extract::{ConnectInfo, FromRequest, Request, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use opencanvas_storage::ViewerSession;
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::{IpAddr, SocketAddr};

const COOKIE: &str = "__Host-opencanvas_viewer";

pub(crate) enum Access {
    Open,
    Session(Proof),
    Locked,
}

#[derive(Clone)]
pub(crate) struct Proof {
    id: String,
    digest: Vec<u8>,
    ip: String,
}

pub(crate) struct Presented {
    token: String,
    ip: IpAddr,
}

pub(crate) fn presented(public: &PublicViewer, request: &Request) -> Option<Presented> {
    let token = cookie(request.headers())?;
    if token.len() > 2048 {
        return None;
    }
    Some(Presented {
        token: token.to_owned(),
        ip: client_ip(public, request)?,
    })
}

impl Proof {
    pub(crate) async fn check(&self, state: &AppState) -> Result<bool> {
        let Self { id, digest, ip } = self.clone();
        state
            .db(move |store| auth(&store)?.check(id, digest, ip, now()))
            .await
    }
}

pub(crate) async fn access(
    state: &AppState,
    public: &PublicViewer,
    presented: Option<Presented>,
) -> Result<Access> {
    let Some(config) = config(state).await? else {
        return Ok(Access::Open);
    };
    let Some(Presented { token, ip }) = presented else {
        return Ok(Access::Locked);
    };
    let Some(claims) = crypto::verify(&token, public.origin(), &config.signing_key) else {
        return Ok(Access::Locked);
    };
    let proof = Proof {
        id: claims.jti.to_string(),
        digest: crypto::digest(&token),
        ip: ip.to_string(),
    };
    Ok(if proof.check(state).await? {
        Access::Session(proof)
    } else {
        Access::Locked
    })
}

fn client_ip(public: &PublicViewer, request: &Request) -> Option<IpAddr> {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()?
        .0
        .ip()
        .to_canonical();
    if public.trusted_proxy != Some(peer) {
        return Some(peer);
    }
    let mut values = request.headers().get_all("x-forwarded-for").iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    let ip = value.parse::<IpAddr>().ok()?.to_canonical();
    (!ip.is_unspecified()).then_some(ip)
}

fn cookie(headers: &HeaderMap) -> Option<&str> {
    let mut found = None;
    for header in headers.get_all(header::COOKIE) {
        for pair in header.to_str().ok()?.split(';') {
            if let Some((COOKIE, value)) = pair.trim().split_once('=') {
                if found.replace(value).is_some() {
                    return None;
                }
            }
        }
    }
    found
}

fn status(authenticated: bool, required: bool) -> Json<Value> {
    Json(json!({"authenticated":authenticated,"required":required}))
}

pub(crate) async fn local_status() -> impl IntoResponse {
    ([(header::CACHE_CONTROL, "no-store")], status(true, false))
}

pub(crate) async fn public_status(
    State(state): State<AppState>,
    Extension(public): Extension<PublicViewer>,
    request: Request,
) -> Result<Json<Value>> {
    Ok(
        match access(&state, &public, presented(&public, &request)).await? {
            Access::Open => status(true, false),
            Access::Session(_) => status(true, true),
            Access::Locked => status(false, true),
        },
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Login {
    password: String,
}

pub(crate) async fn login(
    State(state): State<AppState>,
    Extension(public): Extension<PublicViewer>,
    request: Request,
) -> Result<Response> {
    let Some(config) = config(&state).await? else {
        return Ok(status(true, false).into_response());
    };
    let Some(ip) = client_ip(&public, &request) else {
        return Ok(failure(
            StatusCode::FORBIDDEN,
            "CLIENT_IP_REQUIRED",
            "A valid client IP is required",
        ));
    };
    let user_agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .chars()
        .take(512)
        .collect();
    let Json(Login { password }) = Json::from_request(request, &state)
        .await
        .map_err(|_| ApiError::invalid("A JSON password is required"))?;
    if !valid_password(&password) {
        return Ok(unauthorized());
    }
    let Some(permit) = state.viewer_auth_limits.admit(ip) else {
        let mut response = failure(
            StatusCode::TOO_MANY_REQUESTS,
            "LOGIN_THROTTLED",
            "Too many login attempts",
        );
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, "60".parse().unwrap());
        return Ok(response);
    };
    let hash = config.password_hash;
    let hash = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        crypto::verify_password(&password, &hash).map(|valid| valid.then_some(hash))
    })
    .await
    .map_err(|_| ApiError::new("INTERNAL_ERROR", "Password verification failed"))??;
    let Some(hash) = hash else {
        return Ok(unauthorized());
    };
    let (token, claims) = crypto::sign(public.origin(), &config.signing_key)?;
    let session = ViewerSession {
        session_id: claims.jti.to_string(),
        ip_address: ip.to_string(),
        user_agent,
        issued_at: claims.iat,
        expires_at: claims.exp,
    };
    let digest = crypto::digest(&token);
    if !state
        .db(move |store| auth(&store)?.issue(hash, session, digest))
        .await?
    {
        return Ok(unauthorized());
    }
    let cookie =
        format!("{COOKIE}={token}; Max-Age={LIFETIME}; Path=/; Secure; HttpOnly; SameSite=Strict");
    Ok(([(header::SET_COOKIE, cookie)], status(true, true)).into_response())
}

pub(crate) fn unauthorized() -> Response {
    failure(
        StatusCode::UNAUTHORIZED,
        "VIEWER_AUTH_REQUIRED",
        "Viewer authentication required",
    )
}

fn failure(status: StatusCode, code: &str, message: &str) -> Response {
    (status, Json(ApiError::new(code, message).value())).into_response()
}
