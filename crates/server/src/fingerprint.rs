use sha2::{Digest, Sha256};
use std::{path::Path, process::Command};

// Keep these policies explicit even when dependency versions do not change.
const NATIVE_ENGINE_REVISION: &str = "opencanvas-native:scene-3/web-4:engine-1";
const NATIVE_TEXT_POLICY_REVISION: &str = "native-text-policy:scene-3/web-4:1";
const NATIVE_RASTER_IDENTITY: &str = "native-rust:tiny-skia:cpu-premultiplied-rgba";

/// Native cache identity uses only compiled inputs and this executable, never Node,
/// Playwright, layout-step, workspace sources or the host's font directories.
pub fn native_pipeline() -> String {
    let mut hash = Sha256::new();
    for identity in [
        NATIVE_ENGINE_REVISION,
        NATIVE_TEXT_POLICY_REVISION,
        NATIVE_RASTER_IDENTITY,
    ] {
        hash.update(identity.as_bytes());
        hash.update([0]);
    }
    hash.update(include_bytes!("../../../Cargo.lock"));
    hash.update([0]);
    // Never silently reuse an identity that omits the compiled implementation.
    // Discard OS errors so even failure cannot expose paths or other host details.
    let executable = executable_digest()
        .unwrap_or_else(|_| panic!("Native pipeline requires a readable executable"));
    hash.update(executable);
    format!("{:x}", hash.finalize())
}

fn executable_digest() -> std::io::Result<[u8; 32]> {
    use std::io::Read;

    let mut executable = std::fs::File::open(std::env::current_exe()?)?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let length = executable.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        hash.update(&buffer[..length]);
    }
    Ok(hash.finalize().into())
}

/// Cache is process-local. Capture installed pipeline/runtime identity once at startup.
/// Replacing runtime binaries or pipeline sources requires restarting the backend.
pub fn pipeline(workspace: &Path) -> String {
    let mut hash = Sha256::new();
    for file in [
        "package-lock.json",
        "Cargo.lock",
        "worker/cli.mjs",
        "worker/pipeline.mjs",
        "worker/runtime.mjs",
        "renderer/dist/index.js",
    ] {
        if let Ok(bytes) = std::fs::read(workspace.join(file)) {
            hash.update(bytes);
        } else {
            hash.update(format!("unavailable:{file}"));
        }
    }
    // Match the worker's explicit binary override; direct/Docker starts retain the debug path.
    let layout = std::env::var_os("OPENCANVAS_LAYOUT_BIN")
        .unwrap_or_else(|| "target/debug/layout-step".into());
    if let Ok(bytes) = std::fs::read(workspace.join(&layout)) {
        hash.update(bytes);
    } else {
        hash.update(format!("unavailable:{}", layout.to_string_lossy()));
    }
    let runtime=Command::new("node").current_dir(workspace)
        .env("PLAYWRIGHT_BROWSERS_PATH",std::env::var_os("PLAYWRIGHT_BROWSERS_PATH").unwrap_or_else(||workspace.join(".runtime/browsers").into_os_string()))
        .args(["--input-type=module","-e",r#"
import { chromium } from 'playwright';
import { statSync,readdirSync } from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
const require=createRequire(import.meta.url);
const executables=[process.execPath,chromium.executablePath()];
try {
 const root=process.env.PLAYWRIGHT_BROWSERS_PATH;
 for (const entry of readdirSync(root,{recursive:true})) {
   if (/(^|\/)(chrome|chrome-headless-shell|Chromium|Google Chrome for Testing)(\.exe)?$/.test(entry)) executables.push(path.join(root,entry));
 }
} catch {}
const files=[...new Set(executables)].sort().map(file=>{
  try {const s=statSync(file);return [file,s.size,s.mtimeMs];}
  catch {return [file,'missing'];}
});
console.log(JSON.stringify({node:process.version,playwright:require('playwright/package.json').version,files}));
"#]).output();
    if let Ok(output) = runtime {
        hash.update(output.stdout);
        hash.update([output.status.success() as u8]);
    }
    format!("{:x}", hash.finalize())
}
