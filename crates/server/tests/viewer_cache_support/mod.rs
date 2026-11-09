use super::{public_support::PublicServer, support::Server};
use reqwest::{Method, RequestBuilder};
use std::{fs, path::PathBuf};
use tempfile::TempDir;

pub const OLD: &str = "<html><script src='/assets/index-OLD12345.js'></script></html>";
pub const NEW: &str = "<html><script src='/assets/index-NEW12345.js'></script></html>";
pub const HASHED: &str = "/assets/index-B0w-Y3c9.js";
pub const CSS: &str = "/assets/index-D4a_1234.css";
pub const WASM: &str = "/scene-wasm/opencanvas_scene_bg.wasm";

pub struct Fixture {
    pub local: Server,
    pub public: PublicServer,
    workspace: TempDir,
}

impl Fixture {
    pub async fn start() -> Self {
        let workspace = tempfile::tempdir().unwrap();
        let dist = workspace.path().join("web/dist");
        fs::create_dir_all(dist.join("assets")).unwrap();
        fs::create_dir_all(dist.join("scene-wasm")).unwrap();
        fs::write(dist.join("index.html"), OLD).unwrap();
        for (path, bytes) in [
            (HASHED, "export const version = 1;"),
            (CSS, "body { color: black; }"),
            (WASM, "old-wasm"),
            ("/assets/stable.js", "old-stable"),
            ("/assets/stable.css", "body {}"),
            ("/assets/index-toolong123.js", "long-hash"),
            ("/assets/index-1234567.js", "short-hash"),
            ("/assets/index-12345678.wasm", "named-wasm"),
            ("/scene-wasm/index-12345678.js", "stable-path-js"),
            ("/assets/index-12345678.js.map", "{}"),
            ("/assets/index-B0w-Y3c9.js.br", "brotli-representation"),
            ("/assets/index-B0w-Y3c9.js.gz", "gzip-representation"),
        ] {
            fs::write(dist.join(&path[1..]), bytes).unwrap();
        }
        let local = Server::configured(|config| config.workspace = workspace.path().into()).await;
        let public = PublicServer::start(local.state.clone()).await;
        Self {
            local,
            public,
            workspace,
        }
    }

    pub fn request(&self, public: bool, method: Method, path: &str) -> RequestBuilder {
        if public {
            self.public.request(method, path)
        } else {
            self.local
                .client
                .request(method, format!("{}{path}", self.local.url))
        }
    }

    pub fn replace_preserving_mtime(&self, path: &str, content: &str) {
        let file = self.dist().join(&path[1..]);
        let modified = fs::metadata(&file).unwrap().modified().unwrap();
        fs::write(&file, content).unwrap();
        fs::File::options()
            .write(true)
            .open(file)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(modified))
            .unwrap();
    }

    fn dist(&self) -> PathBuf {
        self.workspace.path().join("web/dist")
    }
}
