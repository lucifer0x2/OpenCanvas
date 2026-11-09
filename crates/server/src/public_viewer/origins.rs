//! Exact public authorities and their HTTPS origins; no cross-domain Origin pairing.
use crate::viewer_origin::ViewerOrigin;
use axum::http::{header, HeaderMap, Uri};
use std::net::IpAddr;

#[cfg(test)]
mod tests;

const MAX_ORIGINS: usize = 8;
const INVALID: &str = "Public viewer origin must be an exact HTTPS origin without a path";

#[derive(Clone, Debug)]
struct AllowedOrigin {
    origin: String,
    authority: String,
}

impl AllowedOrigin {
    fn parse(origin: &str) -> Result<Self, &'static str> {
        let uri: Uri = origin.parse().map_err(|_| INVALID)?;
        let authority = uri.authority().ok_or(INVALID)?;
        let host = authority.host();
        let valid_port = match authority.as_str().strip_prefix(host) {
            Some("") => true,
            Some(suffix) => suffix.strip_prefix(':').is_some_and(|port| {
                !port.is_empty()
                    && port.bytes().all(|byte| byte.is_ascii_digit())
                    && port.parse::<u16>().is_ok_and(|port| port != 0)
            }),
            None => false,
        };
        let valid_host = if host.starts_with('[') && host.ends_with(']') {
            host[1..host.len() - 1].parse::<IpAddr>().is_ok()
        } else {
            host.len() <= 253
                && host.split('.').all(|label| {
                    !label.is_empty()
                        && label.len() <= 63
                        && !label.starts_with('-')
                        && !label.ends_with('-')
                        && label
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                })
        };
        if uri.scheme_str() != Some("https")
            || !valid_host
            || authority.as_str().contains('@')
            || !valid_port
            || origin.contains(['?', '#'])
            || !matches!(uri.path(), "" | "/")
        {
            return Err(INVALID);
        }
        Ok(Self {
            origin: format!("https://{authority}"),
            authority: authority.to_string(),
        })
    }
}

#[derive(Clone, Debug)]
pub(super) struct AllowedOrigins(Vec<AllowedOrigin>);

impl AllowedOrigins {
    pub(super) fn new(origin: &str) -> Result<Self, &'static str> {
        Ok(Self(vec![AllowedOrigin::parse(origin)?]))
    }

    pub(super) fn primary(&self) -> &str {
        &self.0[0].origin
    }

    pub(super) fn additional(mut self, origins: &str) -> Result<Self, &'static str> {
        // Bound parsing work as well as the retained set, even if entries are duplicates.
        if origins.len() > 4096 {
            return Err("Public viewer origin list is too long");
        }
        for origin in origins.split(',') {
            let origin = AllowedOrigin::parse(origin.trim())?;
            if self.0.iter().any(|allowed| allowed.origin == origin.origin) {
                continue;
            }
            if self.0.len() == MAX_ORIGINS {
                return Err("Public viewer permits at most 8 origins");
            }
            self.0.push(origin);
        }
        Ok(self)
    }

    pub(super) fn request(
        &self,
        headers: &HeaderMap,
        login: bool,
    ) -> Result<ViewerOrigin, &'static str> {
        let mut hosts = headers.get_all(header::HOST).iter();
        let host = hosts.next().and_then(|value| value.to_str().ok());
        if hosts.next().is_some() {
            return Err("HOST_REJECTED");
        }
        let matched = self
            .0
            .iter()
            .find(|allowed| Some(allowed.authority.as_str()) == host)
            .ok_or("HOST_REJECTED")?;
        let mut origins = headers.get_all(header::ORIGIN).iter();
        let origin = origins.next();
        if origins.next().is_some()
            || origin.is_some_and(|value| value != matched.origin.as_str())
            || (login && origin.is_none())
        {
            return Err("ORIGIN_REJECTED");
        }
        Ok(ViewerOrigin(matched.origin.clone()))
    }
}
