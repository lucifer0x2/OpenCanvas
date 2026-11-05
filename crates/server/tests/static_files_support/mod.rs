use crate::native_support::Server;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use reqwest::{Client, RequestBuilder};
use std::{
    fs,
    io::{Read, Write},
};

pub const WASM: &[u8] = b"\0asm\x01\0\0\0";
pub const JS: &[u8] = b"export const scene = 'OpenCanvas';\n";
pub const HTML: &[u8] = b"<!doctype html><title>OpenCanvas fixture</title>";

pub async fn fixture() -> Server {
    let server = Server::start().await;
    let dist = server.root.path().join("web/dist");
    fs::create_dir_all(dist.join("assets")).unwrap();
    fs::write(dist.join("index.html"), HTML).unwrap();
    fs::write(dist.join("assets/plain.js"), JS).unwrap();
    for (path, bytes) in [("assets/scene.wasm", WASM), ("assets/scene.js", JS)] {
        fs::write(dist.join(path), bytes).unwrap();
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(bytes).unwrap();
        fs::write(dist.join(format!("{path}.gz")), gzip.finish().unwrap()).unwrap();
        let mut brotli = Vec::new();
        brotli::CompressorWriter::new(&mut brotli, 4096, 5, 22)
            .write_all(bytes)
            .unwrap();
        fs::write(dist.join(format!("{path}.br")), brotli).unwrap();
    }
    // Files matching an API/SSE URL catch an accidental static-route precedence change.
    fs::create_dir_all(dist.join("api/v1")).unwrap();
    for path in ["api/v1/projects", "api/v1/events"] {
        fs::write(dist.join(path), b"wrong static response").unwrap();
        fs::write(
            dist.join(format!("{path}.br")),
            b"wrong compressed response",
        )
        .unwrap();
    }
    server
}

pub fn request(server: &Server, path: &str, accept: &str) -> RequestBuilder {
    client()
        .get(format!("{}{path}", server.url))
        .header("accept-encoding", accept)
}

pub fn head(server: &Server, path: &str, accept: &str) -> RequestBuilder {
    client()
        .head(format!("{}{path}", server.url))
        .header("accept-encoding", accept)
}

fn client() -> Client {
    // Inspect wire bytes even if another test enables reqwest decompression features.
    Client::builder()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .build()
        .unwrap()
}

pub fn unpack(encoding: &str, bytes: &[u8]) -> Vec<u8> {
    let mut original = Vec::new();
    match encoding {
        "gzip" => GzDecoder::new(bytes).read_to_end(&mut original).unwrap(),
        "br" => brotli::Decompressor::new(bytes, 4096)
            .read_to_end(&mut original)
            .unwrap(),
        _ => panic!("unexpected encoding: {encoding}"),
    };
    original
}
