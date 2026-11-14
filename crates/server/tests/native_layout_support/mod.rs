//! Independent full-scene acceptance harness; no product implementation lives here.
pub mod compare;
pub mod fixtures;
pub mod fonts;
pub mod runtime;
use super::mcp::Mcp;
use opencanvas_server::{router, AppState, Config, RenderBackend};
use opencanvas_storage::Storage;
use serde_json::{json, Value};
use std::{path::PathBuf, time::Duration};

pub fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub struct Server {
    pub state: AppState,
    pub connection: Mcp,
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
    pub async fn start(store: Storage, backend: RenderBackend, root: &std::path::Path) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let source = if backend == RenderBackend::Rust {
            root.join("empty-native-workspace")
        } else {
            workspace()
        };
        std::fs::create_dir_all(&source).unwrap();
        // Choose native before fingerprinting, never Config::new then a backend mutation.
        let mut config =
            Config::for_backend(source, listener.local_addr().unwrap().port(), backend);
        if backend == RenderBackend::Rust {
            config.node = root
                .join("node-does-not-exist")
                .to_string_lossy()
                .into_owned();
            assert!(!std::path::Path::new(&config.node).exists());
            assert!(!config.workspace.join("worker").exists());
        }
        let url = config.base_url.clone();
        let state = AppState::new(store, config);
        let app = router(state.clone());
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self {
            state,
            connection: Mcp::connect(url).await,
            task,
        }
    }

    pub async fn scene(
        &self,
        project: &str,
        revision: u64,
        frame: &str,
        width: u64,
        state: &str,
    ) -> Value {
        let request = json!({"projectId":project,"revision":revision,"frameId":frame,
            "viewport":{"width":width,"height":1200},"state":state,"scale":0.25});
        let pending = self.call("render_frame", request.clone()).await;
        assert!(pending["jobId"].is_string(), "{pending}");
        let id = pending["jobId"].as_str().unwrap();
        // Poll the exact requested job, not context's latest frame/viewport cache entry.
        let job = tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                let current = self.get(&format!("/api/v1/jobs/{id}")).await;
                if current["status"] != "pending" {
                    break current;
                }
                tokio::time::sleep(Duration::from_millis(30)).await;
            }
        })
        .await
        .expect("requested scene did not settle");
        assert_eq!(job["status"], "ready", "{job}");
        assert!(
            job["scene"]["nodes"].is_array(),
            "scene must survive strict PNG errors: {job}"
        );
        assert_eq!(job["revision"], revision);
        let provenance = &job["scene"]["provenance"];
        assert_eq!(provenance["projectId"], project);
        assert_eq!(provenance["frameId"], frame);
        assert_eq!(provenance["revision"], revision);
        // The root takes the viewport; a hug-height root reports its computed height.
        let root = &job["scene"]["nodes"][0]["bounds"];
        for (axis, expected) in [("width", &request["viewport"]), ("height", root)] {
            assert_eq!(
                provenance["viewport"][axis].as_f64(),
                expected[axis].as_f64()
            );
        }
        assert_eq!(provenance["state"], state);
        assert_eq!(provenance["sourceSceneKey"], job["sceneKey"]);
        let inspect = self
            .call(
                "inspect_layout",
                json!({"projectId":project,"jobId":id,
            "sceneKey":job["sceneKey"],"limit":200}),
            )
            .await;
        assert_eq!(inspect["revision"], revision, "{inspect}");
        assert_eq!(inspect["sceneKey"], job["sceneKey"]);
        assert_ne!(inspect["truncated"], true, "{inspect}");
        let inspected = inspect["nodes"].as_array().unwrap();
        let nodes = job["scene"]["nodes"].as_array().unwrap();
        assert_eq!(inspected.len(), nodes.len());
        for (actual, full) in inspected.iter().zip(nodes) {
            for field in ["id", "effectiveTheme", "bounds", "lines"] {
                assert_eq!(actual[field], full[field], "inspect vs HTTP {id} {field}");
            }
        }
        let cached = self.call("render_frame", request).await;
        assert_eq!(cached["jobId"], job["jobId"], "cache reuse: {cached}");
        job
    }
}
