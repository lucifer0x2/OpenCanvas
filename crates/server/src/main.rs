mod startup;

use opencanvas_server::{router, AppState, Config};
use std::{net::SocketAddr, time::Duration};

// A stream that still will not end must not keep the process alive; a second Ctrl-C leaves at once.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(3);
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(startup::Startup {
        storage,
        port,
        host,
        public_viewer,
        workspace,
    }) = startup::Startup::from_env()?
    else {
        println!("{}", startup::HELP);
        return Ok(());
    };
    let listener = tokio::net::TcpListener::bind(SocketAddr::from((host, port))).await?;
    let public_listener = match public_viewer {
        Some((port, viewer)) => {
            let listener = tokio::net::TcpListener::bind(SocketAddr::from((host, port))).await?;
            Some((listener, viewer))
        }
        None => None,
    };
    let actual = listener.local_addr()?.port();
    let state = AppState::new(storage.open()?, Config::new(workspace, actual));
    if host.is_unspecified() {
        println!("OpenCanvas http://127.0.0.1:{actual} | MCP /mcp | bound to 0.0.0.0:{actual}");
    } else {
        println!("OpenCanvas http://127.0.0.1:{actual} | MCP /mcp");
    }
    if let Some((listener, public)) = &public_listener {
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
        if let Some((listener, public)) = public_listener {
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
