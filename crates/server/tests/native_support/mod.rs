mod mcp;
pub use mcp::Mcp;
use opencanvas_server::{router, AppState, Config, RenderBackend};
use opencanvas_storage::Store;
use serde_json::{json, Value};
use tempfile::TempDir;

pub struct Server {
    pub root: TempDir,
    pub state: AppState,
    connection: Mcp,
    task: tokio::task::JoinHandle<()>,
}
impl std::ops::Deref for Server {
    type Target = Mcp;
    fn deref(&self) -> &Mcp {
        &self.connection
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.state.begin_shutdown();
        self.task.abort();
    }
}
impl Server {
    pub async fn start() -> Self {
        Self::start_backend(RenderBackend::Rust).await
    }
    pub async fn start_backend(backend: RenderBackend) -> Self {
        Self::configured(backend, |_| {}).await
    }
    pub async fn configured(backend: RenderBackend, configure: impl FnOnce(&mut Config)) -> Self {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path().join("data")).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        // Empty workspace: no Node, layout binary, JS files or bundled font files.
        let mut config = Config::for_backend(
            if backend == RenderBackend::Rust {
                root.path().into()
            } else {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
            },
            listener.local_addr().unwrap().port(),
            backend,
        );
        if backend == RenderBackend::Rust {
            config.node = root.path().join("no-node").to_string_lossy().into_owned();
        }
        configure(&mut config);
        let url = config.base_url.clone();
        let state = AppState::new(store, config);
        let app = router(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let connection = Mcp::connect(url).await;
        Self {
            root,
            state,
            connection,
            task,
        }
    }
}

pub fn design() -> Value {
    json!([
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":320,"height":200,"states":{"hover":{"fill":"#abcdef"}}}},
        {"op":"create_node","id":"text","kind":"text","parentId":"frame","props":{"text":"Private long copy 🦄","textWrap":"nowrap","width":8,"height":8,"fontSize":20,"lineHeight":28,"fontId":"font_bundled"}},
        {"op":"create_note","id":"intent","nodeId":"text","text":"Intentional overflow: display lettering"}
    ])
}
