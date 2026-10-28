use crate::{
    error::{ApiError, Result},
    state::Config,
};
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
};

pub struct Worker {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    sequence: u64,
}
impl Worker {
    pub fn start(config: &Config) -> Result<Self> {
        let mut child = Command::new(&config.node)
            .arg("worker/cli.mjs")
            .current_dir(&config.workspace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| ApiError::new("WORKER_UNAVAILABLE", "Render process could not start"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ApiError::new("WORKER_FAILED", "Worker input unavailable"))?;
        let stdout = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| ApiError::new("WORKER_FAILED", "Worker output unavailable"))?,
        );
        Ok(Self {
            child,
            stdin,
            stdout,
            sequence: 0,
        })
    }
    pub async fn call(&mut self, mut request: Value) -> Result<Value> {
        self.sequence += 1;
        request["id"] = json!(self.sequence);
        let mut bytes = serde_json::to_vec(&request)?;
        bytes.push(b'\n');
        self.stdin.write_all(&bytes).await.map_err(|_| failure())?;
        let mut line = String::new();
        // Read at most 48MiB including a bounded PNG base64 response.
        let read = (&mut self.stdout)
            .take(48 * 1024 * 1024)
            .read_line(&mut line)
            .await
            .map_err(|_| failure())?;
        if read == 0 || !line.ends_with('\n') {
            return Err(failure());
        }
        let response: Value = serde_json::from_str(&line).map_err(|_| failure())?;
        if response["id"] != self.sequence || response["status"] != "ready" {
            let code = response["code"]
                .as_str()
                .filter(|s| s.len() <= 64 && s.bytes().all(|c| c.is_ascii_uppercase() || c == b'_'))
                .unwrap_or("WORKER_FAILED");
            return Err(ApiError::new(
                code,
                "Render worker could not complete this representation",
            ));
        }
        Ok(response)
    }
    pub async fn stop(&mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}
fn failure() -> ApiError {
    ApiError::new(
        "WORKER_FAILED",
        "Render worker exited or returned an invalid response",
    )
}
use tokio::io::AsyncReadExt;
