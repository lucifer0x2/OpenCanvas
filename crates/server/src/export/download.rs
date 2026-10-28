//! `GET /api/v1/jobs/{job}/files/{index}`: one export file as an attachment named by its
//! `fileName`, ASCII-safe in `filename` and exact in RFC 5987 `filename*`.
use crate::{error::Result, state::AppState};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue},
    response::Response,
};

pub(crate) async fn file(
    State(s): State<AppState>,
    Path((job, index)): Path<(String, usize)>,
) -> Result<Response> {
    let file = s.jobs.file(&job, index).await?;
    let mime = file.meta["mimeType"]
        .as_str()
        .unwrap_or("application/octet-stream");
    let name = file.meta["fileName"].as_str().unwrap_or("export");
    let mut response = Response::new(Body::from(file.bytes));
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(mime) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    if let Ok(value) = HeaderValue::from_str(&disposition(name)) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    Ok(response)
}

fn disposition(name: &str) -> String {
    let ascii: String = name
        .chars()
        .map(|c| match c {
            ' '..='~' if c != '"' && c != '\\' => c,
            _ => '_',
        })
        .collect();
    let mut encoded = String::new();
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~@".contains(&byte) {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn disposition_keeps_ascii_and_encodes_utf8() {
        assert_eq!(
            super::disposition("Nút \"tìm\"@2x.png"),
            "attachment; filename=\"N_t _t_m_@2x.png\"; filename*=UTF-8''N%C3%BAt%20%22t%C3%ACm%22@2x.png"
        );
    }
}
