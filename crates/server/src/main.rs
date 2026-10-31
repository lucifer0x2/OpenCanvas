use opencanvas_server::{router, AppState, Config, PublicViewer};
use opencanvas_storage::Store;
use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

// A stream that still will not end must not keep the process alive; a second Ctrl-C leaves at once.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(3);
const HELP: &str =
    "opencanvas-server [--data-root DIR] [--port 3847] [--workspace DIR] [--host 127.0.0.1|0.0.0.0]
[--public-viewer-port 3848 --public-viewer-origin https://machine.tailnet.ts.net]
Default data root: ~/.opencanvas. Creates no project until create_project.
--host defaults to 127.0.0.1. 0.0.0.0 listens on every interface and is meant for a container
whose ports are published on host loopback only: the API has no authentication.
Public viewer is disabled by default; both public options are required. This separate listener
shares ALL projects, live events, resources and exports publicly, without MCP or write access.
Point Funnel only at the public viewer port, NEVER at the local MCP port.";

/// Render jobs, worker resources and image URLs call back on `http://127.0.0.1:<port>`
/// (`Config::base_url`), so only bind addresses that include IPv4 loopback are accepted.
fn bind_host(value: &str) -> Result<Ipv4Addr, Box<dyn std::error::Error>> {
    match value {
        "127.0.0.1" => Ok(Ipv4Addr::LOCALHOST),
        "0.0.0.0" => Ok(Ipv4Addr::UNSPECIFIED),
        _ => Err("--host must be 127.0.0.1 or 0.0.0.0".into()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut data_root = std::env::var_os("OPENCANVAS_DATA_ROOT").map(PathBuf::from);
    let mut port = 3847;
    let mut host = Ipv4Addr::LOCALHOST;
    let mut public_port: Option<u16> = None;
    let mut public_origin = None;
    let mut workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-root" => {
                data_root = Some(PathBuf::from(
                    args.next().ok_or("--data-root needs a directory")?,
                ))
            }
            "--port" => port = args.next().ok_or("--port needs a number")?.parse()?,
            "--host" => host = bind_host(&args.next().ok_or("--host needs an address")?)?,
            "--public-viewer-port" => {
                public_port = Some(
                    args.next()
                        .ok_or("--public-viewer-port needs a number")?
                        .parse()?,
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
            "--help" => {
                println!("{HELP}");
                return Ok(());
            }
            _ => return Err("Unknown server option".into()),
        }
    }
    if public_port.is_some() != public_origin.is_some() {
        return Err(
            "--public-viewer-port and --public-viewer-origin must be supplied together".into(),
        );
    }
    if public_port.is_some_and(|p| p != 0 && p == port) {
        return Err("Local and public viewer ports must differ".into());
    }
    let root = data_root
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".opencanvas")))
        .ok_or("Data root is required")?;
    let listener = tokio::net::TcpListener::bind(SocketAddr::from((host, port))).await?;
    let public_listener = if let Some(public_port) = public_port {
        Some(tokio::net::TcpListener::bind(SocketAddr::from((host, public_port))).await?)
    } else {
        None
    };
    let actual = listener.local_addr()?.port();
    let store = Store::open(root)?;
    let state = AppState::new(store, Config::new(workspace, actual));
    if host.is_unspecified() {
        println!("OpenCanvas http://127.0.0.1:{actual} | MCP /mcp | bound to 0.0.0.0:{actual}");
    } else {
        println!("OpenCanvas http://127.0.0.1:{actual} | MCP /mcp");
    }
    if let (Some(listener), Some(public)) = (&public_listener, &public_origin) {
        println!(
            "Public viewer {} | bound to {host}:{} | ALL projects shared; no MCP or writes",
            public.origin(),
            listener.local_addr()?.port()
        );
    }
    let stopping = state.clone();
    let signal = tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        println!("Stopping…");
        stopping.begin_shutdown();
        tokio::spawn(async {
            tokio::select! {
                _ = tokio::time::sleep(SHUTDOWN_GRACE) => {}
                _ = tokio::signal::ctrl_c() => {}
            }
            std::process::exit(0);
        });
    });
    let local_state = state.clone();
    let local = axum::serve(listener, router(state.clone()))
        .with_graceful_shutdown(async move { local_state.shutting_down().await });
    let public = async move {
        if let (Some(listener), Some(public)) = (public_listener, public_origin) {
            let stopping = state.clone();
            axum::serve(listener, public.router(state))
                .with_graceful_shutdown(async move { stopping.shutting_down().await })
                .await?;
        }
        Ok::<(), std::io::Error>(())
    };
    let result = tokio::try_join!(async { local.await }, public);
    signal.abort();
    result?;
    Ok(())
}
