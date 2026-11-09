use opencanvas_server::RenderBackend;
use std::{
    env,
    net::{IpAddr, Ipv4Addr},
};

pub(super) fn render_backend(cli: Option<RenderBackend>) -> Result<RenderBackend, &'static str> {
    const INVALID: &str = "RENDER_BACKEND must be chromium or rust";
    if let Some(backend) = cli {
        return Ok(backend);
    }
    match env::var("RENDER_BACKEND") {
        Ok(value) => value.parse().map_err(|_| INVALID),
        Err(env::VarError::NotPresent) => Ok(RenderBackend::Chromium),
        Err(_) => Err(INVALID),
    }
}

pub(super) fn trusted_proxy() -> Result<Option<IpAddr>, &'static str> {
    const INVALID: &str = "OPENCANVAS_PUBLIC_TRUSTED_PROXY_IP must be a single IP address";
    match env::var("OPENCANVAS_PUBLIC_TRUSTED_PROXY_IP") {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => value
            .parse::<IpAddr>()
            .ok()
            .map(|ip| ip.to_canonical())
            .filter(|ip| !ip.is_unspecified())
            .map(Some)
            .ok_or(INVALID),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(_) => Err(INVALID),
    }
}

pub(super) fn env_port() -> Result<u16, &'static str> {
    const INVALID: &str = "PORT must be a number from 0 to 65535";
    match env::var("PORT") {
        Ok(value) => value.parse().map_err(|_| INVALID),
        Err(env::VarError::NotPresent) => Ok(3847),
        Err(_) => Err(INVALID),
    }
}

/// Worker callbacks require IPv4 loopback to be included in the bind address.
pub(super) fn bind_host(value: &str) -> Result<Ipv4Addr, &'static str> {
    match value {
        "127.0.0.1" => Ok(Ipv4Addr::LOCALHOST),
        "0.0.0.0" => Ok(Ipv4Addr::UNSPECIFIED),
        _ => Err("--host must be 127.0.0.1 or 0.0.0.0"),
    }
}
