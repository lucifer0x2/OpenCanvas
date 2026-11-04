//! Record checked source bytes, reject stale Chromium renderer/pins before comparison.
use super::workspace;
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn check() -> Vec<(String, String)> {
    eprintln!(
        "RUN_START unix_seconds={}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    eprintln!(
        "TEST_EXECUTABLE sha256={:x}",
        Sha256::digest(std::fs::read(std::env::current_exe().unwrap()).unwrap())
    );
    let result = std::process::Command::new("node").current_dir(workspace())
        .args(["--input-type=module", "-e", r#"
import { playwrightRuntime, PIN } from './spikes/native-render-parity/environment.mjs';
import { build } from 'esbuild';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
await playwrightRuntime();
const bundle = await build({entryPoints:['renderer/index.ts'],bundle:true,format:'esm',write:false});
const hash = b => createHash('sha256').update(b).digest('hex');
if (hash(bundle.outputFiles[0].contents) !== hash(await readFile('renderer/dist/index.js')))
  throw new Error('Stale renderer: run npm run build:renderer before acceptance');
console.log(JSON.stringify({node:process.version,platform:process.platform,arch:process.arch,pins:PIN}));
"#]).output().expect("Chromium parity requires Node");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    eprintln!("RUNTIME {}", String::from_utf8_lossy(&result.stdout).trim());
    let mut paths = Vec::new();
    // Guard scene-computation paths. The separately owned painter may change during
    // acceptance; no raster claim is made, and the executable digest records it.
    for directory in [
        "crates/text/src",
        "crates/layout/src",
        "crates/server/src/jobs",
        "renderer",
        "worker",
    ] {
        source_paths(&workspace().join(directory), &mut paths);
    }
    for file in [
        "Cargo.lock",
        "crates/server/src/state.rs",
        "crates/server/src/fingerprint.rs",
        "renderer/dist/index.js",
    ] {
        paths.push(workspace().join(file));
    }
    paths.sort();
    paths.dedup();
    let manifest: Vec<_> = paths
        .into_iter()
        .map(|path| {
            let hash = format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap()));
            (
                path.strip_prefix(workspace())
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                hash,
            )
        })
        .collect();
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&manifest).unwrap())
    );
    eprintln!("CHECKED_SOURCES sha256={digest} files={}", manifest.len());
    eprintln!(
        "SOURCE_MANIFEST {}",
        serde_json::to_string(&manifest).unwrap()
    );
    manifest
}

fn source_paths(path: &Path, result: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if !matches!(
                path.file_name().and_then(|n| n.to_str()),
                Some("dist" | "fonts" | "node_modules")
            ) {
                source_paths(&path, result);
            }
        } else if matches!(
            path.extension().and_then(|n| n.to_str()),
            Some("rs" | "mjs" | "ts")
        ) {
            result.push(path);
        }
    }
}

pub fn unchanged(manifest: &[(String, String)]) {
    for (path, expected) in manifest {
        let hash = format!(
            "{:x}",
            Sha256::digest(std::fs::read(workspace().join(path)).unwrap())
        );
        assert_eq!(
            &hash, expected,
            "Source changed during run: {path}; rerun against settled sources"
        );
    }
    eprintln!(
        "RUN_END unix_seconds={} checked_sources_unchanged=true",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
}
