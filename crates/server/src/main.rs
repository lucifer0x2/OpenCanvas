use opencanvas_server::{router, AppState, Config};
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
Default data root: ~/.opencanvas. Creates no project until create_project.
--host defaults to 127.0.0.1. 0.0.0.0 listens on every interface and is meant for a container
whose port is published on host loopback only: the API has no authentication.";

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
    let root = data_root
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".opencanvas")))
        .ok_or("Data root is required")?;
    let store = Store::open(root)?;
    let listener = tokio::net::TcpListener::bind(SocketAddr::from((host, port))).await?;
    let actual = listener.local_addr()?.port();
    let state = AppState::new(store, Config::new(workspace, actual));
    if host.is_unspecified() {
        println!("OpenCanvas http://127.0.0.1:{actual} | MCP /mcp | bound to 0.0.0.0:{actual}");
    } else {
        println!("OpenCanvas http://127.0.0.1:{actual} | MCP /mcp");
    }
    let stopping = state.clone();
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async move {
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
        })
        .await?;
    Ok(())
}
