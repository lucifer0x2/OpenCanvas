//! Minimal bounded HTTP framing for the local spike; no production ingress/auth.
use oc_spike_core::DomainError;
use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub struct Request {
    pub method: String,
    pub path: String,
    pub body: Vec<u8>,
    pub drop_response: bool,
}

fn invalid(error: impl std::fmt::Display) -> DomainError {
    DomainError::new("VALIDATION_FAILED", error.to_string())
}

pub async fn read(socket: &mut TcpStream) -> Result<Request, DomainError> {
    let mut bytes = Vec::new();
    let split = loop {
        if let Some(index) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break index + 4;
        }
        if bytes.len() > 16_384 {
            return Err(invalid("Headers exceed limit"));
        }
        let mut buf = [0; 4096];
        let count = socket.read(&mut buf).await.map_err(invalid)?;
        if count == 0 {
            return Err(invalid("Unexpected EOF"));
        }
        bytes.extend_from_slice(&buf[..count]);
    };
    let header = std::str::from_utf8(&bytes[..split]).map_err(invalid)?;
    let mut lines = header.lines();
    let first: Vec<_> = lines.next().unwrap_or("").split_whitespace().collect();
    if first.len() != 3 {
        return Err(invalid("Malformed HTTP request"));
    }
    let (method, path) = (first[0].to_owned(), first[1].to_owned());
    let mut length = 0;
    let mut drop_response = false;
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            match key.to_ascii_lowercase().as_str() {
                "content-length" => length = value.trim().parse::<usize>().map_err(invalid)?,
                "transfer-encoding" => {
                    return Err(invalid("Chunked requests unsupported by spike"))
                }
                "x-spike-drop-response" => drop_response = value.trim() == "1",
                _ => {}
            }
        }
    }
    if length > crate::blobs::MAX_BYTES {
        return Err(invalid("Body exceeds 2 MiB"));
    }
    while bytes.len() < split + length {
        let mut buf = [0; 8192];
        let count = socket.read(&mut buf).await.map_err(invalid)?;
        if count == 0 {
            return Err(invalid("Unexpected body EOF"));
        }
        bytes.extend_from_slice(&buf[..count]);
    }
    Ok(Request {
        method,
        path,
        body: bytes[split..split + length].to_vec(),
        drop_response,
    })
}

pub async fn send_error(socket: &mut TcpStream, error: DomainError) -> std::io::Result<()> {
    let status = match error.code.as_str() {
        "PROJECT_NOT_FOUND" | "NOT_FOUND" => 404,
        "REVISION_CONFLICT" | "IDEMPOTENCY_KEY_REUSED" | "PROJECT_EXISTS" | "STALE_ATTEMPT" => 409,
        "INTERNAL_ERROR" | "INTEGRITY_ERROR" => 500,
        _ => 400,
    };
    send(socket, status, json!({"error":error})).await
}

pub async fn send(socket: &mut TcpStream, status: u16, body: Value) -> std::io::Result<()> {
    send_bytes(socket, status, body.to_string().as_bytes()).await
}

pub async fn send_bytes(socket: &mut TcpStream, status: u16, body: &[u8]) -> std::io::Result<()> {
    let header = format!("HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
    socket.write_all(header.as_bytes()).await?;
    socket.write_all(body).await?;
    socket.shutdown().await
}
