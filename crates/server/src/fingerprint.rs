use sha2::{Digest, Sha256};
use std::{path::Path, process::Command};

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
        "target/debug/layout-step",
    ] {
        if let Ok(bytes) = std::fs::read(workspace.join(file)) {
            hash.update(bytes);
        } else {
            hash.update(format!("unavailable:{file}"));
        }
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
