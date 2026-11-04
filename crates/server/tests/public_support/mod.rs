use opencanvas_server::{AppState, PublicViewer};
use reqwest::{Client, Method, RequestBuilder};
use serde_json::Value;

pub const ORIGIN: &str = "https://canvas.example.ts.net";
pub const HOST: &str = "canvas.example.ts.net";

pub async fn set_privacy(local: &super::support::Server, project: &str, privacy: &str) -> Value {
    let updated = local
        .call(
            "update_project",
            serde_json::json!({"projectId":project,"privacy_status":privacy}),
        )
        .await;
    assert_eq!(updated["privacy_status"], privacy, "{updated}");
    updated
}

pub struct PublicServer {
    pub url: String,
    pub client: Client,
    task: tokio::task::JoinHandle<()>,
}

impl PublicServer {
    pub async fn start(state: AppState) -> Self {
        Self::start_with(state, PublicViewer::new(ORIGIN).unwrap()).await
    }

    pub async fn start_with(state: AppState, viewer: PublicViewer) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let app = viewer.router(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .with_graceful_shutdown(async move { state.shutting_down().await })
            .await
            .unwrap();
        });
        Self {
            url,
            client: Client::new(),
            task,
        }
    }

    pub fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.client
            .request(method, format!("{}{path}", self.url))
            .header("host", HOST)
            .header("origin", ORIGIN)
    }

    pub async fn get(&self, path: &str) -> Value {
        let response = self.request(Method::GET, path).send().await.unwrap();
        assert_eq!(response.status(), 200, "{path}");
        response.json().await.unwrap()
    }

    pub async fn resource(&self, url: &Value) -> reqwest::Response {
        let path = url
            .as_str()
            .unwrap()
            .strip_prefix(ORIGIN)
            .expect("Public origin link");
        let response = self.request(Method::GET, path).send().await.unwrap();
        assert_eq!(response.status(), 200, "{path}");
        response
    }
}

impl Drop for PublicServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
