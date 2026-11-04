mod options;
mod postgres_env;

use opencanvas_server::{PublicViewer, RenderBackend};
use opencanvas_storage::{PostgresBackend, PostgresConfig, SnapshotCache, Storage, Store};
use options::{bind_host, env_port, render_backend, trusted_proxy};
use std::{env, error::Error, net::Ipv4Addr, path::PathBuf};

pub const HELP: &str =
    "opencanvas-server [--data-root DIR] [--port NUMBER] [--workspace DIR] [--host 127.0.0.1|0.0.0.0]
[--public-viewer-port 3848 --public-viewer-origin https://machine.tailnet.ts.net]
[--render-backend chromium|rust]
RENDER_BACKEND defaults to chromium. --render-backend overrides RENDER_BACKEND.
DATABASE_PROVIDER defaults to sqlite. Use postgresql with DATABASE_URL to enable PostgreSQL
with an in-process snapshot cache (1 hour TTL). PostgreSQL ignores the data root.
Optional PostgreSQL limits: DATABASE_POOL_SIZE (1-32, default 4) and, in milliseconds,
DATABASE_CONNECT_TIMEOUT_MS (3000), DATABASE_STATEMENT_TIMEOUT_MS (15000),
DATABASE_LOCK_TIMEOUT_MS (3000), DATABASE_OPERATION_TIMEOUT_MS (30000).
SQLite data root defaults to ~/.opencanvas. Neither backend creates a project until create_project.
--port overrides PORT (default 3847). Use 0 to bind an available port.
--host defaults to 127.0.0.1. 0.0.0.0 listens on every interface and is meant for a container
whose ports are published on host loopback only: the API has no authentication.
Public viewer is disabled by default; both public options are required. This separate listener
shares public projects, their live events, resources and exports. Projects default to private.
Local MCP can require a password for the public viewer. Behind a trusted reverse proxy set
OPENCANVAS_PUBLIC_TRUSTED_PROXY_IP to its socket peer IP to use a single X-Forwarded-For address.
Point Funnel only at the public viewer port, NEVER at the local MCP port.";

pub struct Startup {
    pub storage: StorageOptions,
    pub port: u16,
    pub host: Ipv4Addr,
    pub public_viewer: Option<(u16, PublicViewer)>,
    pub workspace: PathBuf,
    pub render_backend: RenderBackend,
}

impl Startup {
    /// Returns no startup configuration when help was requested.
    pub fn from_env() -> Result<Option<Self>, Box<dyn Error>> {
        if env::args().skip(1).any(|arg| arg == "--help") {
            return Ok(None);
        }
        let mut data_root = env::var_os("OPENCANVAS_DATA_ROOT").map(PathBuf::from);
        let mut port = None;
        let mut host = Ipv4Addr::LOCALHOST;
        let mut public_port = None;
        let mut public_origin = None;
        let mut backend = None;
        let mut workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--data-root" => {
                    data_root = Some(PathBuf::from(
                        args.next().ok_or("--data-root needs a directory")?,
                    ))
                }
                "--port" => {
                    port = Some(
                        args.next()
                            .ok_or("--port needs a number")?
                            .parse()
                            .map_err(|_| "--port must be a number from 0 to 65535")?,
                    )
                }
                "--host" => host = bind_host(&args.next().ok_or("--host needs an address")?)?,
                "--public-viewer-port" => {
                    public_port = Some(
                        args.next()
                            .ok_or("--public-viewer-port needs a number")?
                            .parse()
                            .map_err(|_| "--public-viewer-port must be a number from 0 to 65535")?,
                    )
                }
                "--public-viewer-origin" => {
                    public_origin = Some(PublicViewer::new(
                        &args
                            .next()
                            .ok_or("--public-viewer-origin needs an HTTPS origin")?,
                    )?)
                }
                "--workspace" => {
                    workspace = PathBuf::from(args.next().ok_or("--workspace needs a directory")?)
                }
                "--render-backend" => {
                    backend = Some(
                        args.next()
                            .ok_or("--render-backend needs chromium or rust")?
                            .parse()
                            .map_err(|_| "--render-backend must be chromium or rust")?,
                    )
                }
                _ => return Err("Unknown server option".into()),
            }
        }

        let render_backend = render_backend(backend)?;
        let port = match port {
            Some(port) => port,
            None => env_port()?,
        };
        let public_viewer = match (public_port, public_origin) {
            (Some(p), Some(_)) if p != 0 && p == port => {
                return Err("Local and public viewer ports must differ".into());
            }
            (Some(p), Some(viewer)) => Some((
                p,
                match trusted_proxy()? {
                    Some(proxy) => viewer.with_trusted_proxy(proxy),
                    None => viewer,
                },
            )),
            (None, None) => None,
            _ => {
                return Err(
                    "--public-viewer-port and --public-viewer-origin must be supplied together"
                        .into(),
                );
            }
        };

        Ok(Some(Self {
            storage: StorageOptions::from_env(data_root)?,
            port,
            host,
            public_viewer,
            workspace,
            render_backend,
        }))
    }
}

pub enum StorageOptions {
    Sqlite(PathBuf),
    Postgres(PostgresConfig),
}

impl StorageOptions {
    fn from_env(data_root: Option<PathBuf>) -> Result<Self, Box<dyn Error>> {
        let provider = env::var_os("DATABASE_PROVIDER");
        match provider.as_deref() {
            Some(value) if value == "postgresql" => {
                let url = env::var("DATABASE_URL")
                    .ok()
                    .filter(|url| !url.trim().is_empty())
                    .ok_or("DATABASE_URL must be set to a nonempty value for PostgreSQL")?;
                Ok(Self::Postgres(postgres_env::config(url, |name| {
                    env::var_os(name)
                })?))
            }
            Some(value) if value != "sqlite" => {
                Err("DATABASE_PROVIDER must be sqlite or postgresql".into())
            }
            _ => Ok(data_root
                .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".opencanvas")))
                .map(Self::Sqlite)
                .ok_or("Data root is required")?),
        }
    }

    pub fn open(self) -> Result<Storage, Box<dyn Error>> {
        match self {
            Self::Sqlite(root) => Ok(Store::open(root)?.into()),
            Self::Postgres(config) => {
                let backend = PostgresBackend::with_config(config).map_err(|_| {
                    "PostgreSQL startup failed. Check DATABASE_URL and database access"
                })?;
                let cache = SnapshotCache::new(backend.cache_namespace());
                Ok(Storage::new(backend.with_cache(cache)?))
            }
        }
    }
}
