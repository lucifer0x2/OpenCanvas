use opencanvas_server::{AppState, PublicViewer};
use reqwest::{Client, Method, RequestBuilder};
use serde_json::Value;

pub const ORIGIN: &str = "https://canvas.example.ts.net";
pub const HOST: &str = "canvas.example.ts.net";

pub struct PublicServer {
    pub url: String,
    pub client: Client,
    task: tokio::task::JoinHandle<()>,
}

impl PublicServer {
    pub async fn start(state: AppState) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let app = PublicViewer::new(ORIGIN).unwrap().router(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
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
