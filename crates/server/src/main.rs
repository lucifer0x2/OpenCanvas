use opencanvas_server::{router, AppState, Config};
use opencanvas_storage::Store;
use std::{net::SocketAddr, path::PathBuf, time::Duration};

// A stream that still will not end must not keep the process alive; a second Ctrl-C leaves at once.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(3);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut data_root = std::env::var_os("OPENCANVAS_DATA_ROOT").map(PathBuf::from);
    let mut port = 3847;
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
            "--workspace" => {
                workspace = PathBuf::from(args.next().ok_or("--workspace needs a directory")?)
            }
            "--help" => {
                println!("opencanvas-server [--data-root DIR] [--port 3847] [--workspace DIR]\nDefault data root: ~/.opencanvas. Creates no project until create_project.");
                return Ok(());
            }
            _ => return Err("Unknown server option".into()),
        }
    }
    let root = data_root
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".opencanvas")))
        .ok_or("Data root is required")?;
    let store = Store::open(root)?;
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await?;
    let actual = listener.local_addr()?.port();
    let state = AppState::new(store, Config::new(workspace, actual));
    println!("OpenCanvas http://127.0.0.1:{actual} | MCP /mcp");
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
