//! Export jobs: compute the node's root frame like a render, then rasterize only the node's
//! subtree once per PNG setting, or write it as SVG (`export_svg`). The record keeps the files' bytes for `/api/v1/jobs/{job}/files/{i}`
//! and their metadata, never the scene.
use super::{export_svg, export_svg::AssetSource, png, subtree, Entry, Jobs};
use crate::{
    error::{ApiError, Result},
    worker::Worker,
};
use serde::Serialize;
use serde_json::{json, Value};

/// Bytes one export job may hold, the worker's response bound.
const MAX_JOB_BYTES: usize = 48 * 1024 * 1024;

/// One node to export and its files; part of the job key.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSpec {
    pub node_id: String,
    pub settings: Vec<ExportSetting>,
}

/// One output file: a PNG at a fixed scale or a target width/height in pixels, or an SVG.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSetting {
    pub format: String,
    pub scale: Option<f64>,
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub file_name: String,
    pub svg_id_attribute: bool,
}

/// What a job produced after its compute: a frame PNG or a node's export files.
pub enum Rendered {
    Frame(Value),
    Export(Vec<StoredFile>),
}

#[derive(Clone)]
pub struct StoredFile {
    pub meta: Value,
    pub bytes: Vec<u8>,
}

impl ExportSetting {
    /// The exact scale this setting asks for, and the scale to rasterize at: a target width or
    /// height is nudged below the exact ratio when float rounding would make `ceil` overshoot.
    fn scales(&self, width: f64, height: f64) -> Result<(f64, f64)> {
        let (target, size) = match (self.width, self.height) {
            (Some(w), _) => (w as f64, width),
            (_, Some(h)) => (h as f64, height),
            _ => {
                let scale = self.scale.unwrap_or(1.0);
                return Ok((scale, scale));
            }
        };
        let exact = target / size;
        if !(0.01..=4.0).contains(&exact) {
            return Err(ApiError::new(
                "EXPORT_SCALE_RANGE",
                "Target width/height implies a scale outside 0.01–4 for this node",
            ));
        }
        let render = if (size * exact).ceil() > target {
            (target - 1e-6) / size
        } else {
            exact
        };
        Ok((exact, render))
    }
}

/// Code of a failure that leaves the worker healthy, so the runner keeps it.
pub fn keeps_worker(error: &ApiError) -> bool {
    matches!(
        error.0.code.as_str(),
        "EMPTY_BOUNDS" | "EXPORT_SCALE_RANGE" | "PIXEL_BUDGET" | "NODE_NOT_FOUND"
    )
}

/// Files of one node from its computed root frame: PNGs from the worker, SVGs in Rust.
pub async fn render(
    worker: &mut Worker,
    assets: &AssetSource,
    computed: &Value,
    spec: &ExportSpec,
) -> Result<Rendered> {
    let scene = subtree::scene(&computed["scene"], &spec.node_id)?;
    let mut images = None;
    let mut svg_fonts = None;
    let mut files = Vec::with_capacity(spec.settings.len());
    let mut total = 0;
    for setting in &spec.settings {
        let file = if setting.format == "svg" {
            if images.is_none() {
                images = Some(export_svg::images(assets, &scene, &computed["readiness"]).await?);
                svg_fonts = Some(super::export_fonts::embed(assets, &scene).await?);
            }
            let (svg_scene, css) = svg_fonts.as_ref().unwrap();
            export_svg::file(svg_scene, images.as_ref().unwrap(), css, setting)?
        } else {
            png_file(worker, &scene, setting).await?
        };
        total += file.bytes.len();
        if total > MAX_JOB_BYTES {
            return Err(ApiError::new(
                "PIXEL_BUDGET",
                "Export files exceed 48 MiB for one node; use fewer or smaller settings",
            ));
        }
        files.push(file);
    }
    Ok(Rendered::Export(files))
}

async fn png_file(
    worker: &mut Worker,
    scene: &Value,
    setting: &ExportSetting,
) -> Result<StoredFile> {
    let viewport = &scene["provenance"]["viewport"];
    let (width, height) = (
        viewport["width"].as_f64().unwrap_or_default(),
        viewport["height"].as_f64().unwrap_or_default(),
    );
    let (scale, render) = setting.scales(width, height)?;
    let result = worker
        .call(json!({"op":"render","scene":scene,"scale":render,"strict":true}))
        .await?;
    let bytes = png::decode(&result)?;
    let meta = json!({"fileName":setting.file_name,"format":setting.format,"mimeType":"image/png","scale":scale,"width":result["width"],"height":result["height"],"bytes":bytes.len()});
    Ok(StoredFile { meta, bytes })
}

/// Fills a finished export record: file metadata with download URLs, no scene.
pub(super) fn finish(entry: &mut Entry, base: &str, computed: &Value, rendered: Result<Rendered>) {
    let id = entry.value["jobId"].as_str().unwrap_or_default().to_owned();
    entry.value["provenance"] = computed["scene"]["provenance"].clone();
    entry.value["readiness"] = computed["readiness"].clone();
    match rendered {
        Ok(Rendered::Export(files)) => {
            let meta: Vec<Value> = files
                .iter()
                .enumerate()
                .map(|(i, file)| {
                    let mut meta = file.meta.clone();
                    meta["url"] = json!(format!("{base}/api/v1/jobs/{id}/files/{i}"));
                    meta
                })
                .collect();
            entry.value["status"] = json!("ready");
            entry.value["files"] = json!(meta);
            entry.files = files;
        }
        Ok(Rendered::Frame(_)) => fail(entry, ApiError::new("WORKER_FAILED", "Not an export")),
        Err(error) => fail(entry, error),
    }
}

fn fail(entry: &mut Entry, error: ApiError) {
    entry.value["status"] = json!("failed");
    entry.value["error"] = error.value()["error"].clone();
}

impl Jobs {
    /// One file of a ready export job.
    pub async fn file(&self, id: &str, index: usize) -> Result<StoredFile> {
        let table = self.table.lock().await;
        let entry = table
            .entries
            .iter()
            .find(|e| e.value["jobId"] == id)
            .ok_or_else(|| ApiError::new("JOB_NOT_FOUND", "Render job is unavailable"))?;
        if entry.value["status"] != "ready" {
            return Err(ApiError::new(
                "IMAGE_NOT_READY",
                "Export is pending or failed; inspect the job error",
            ));
        }
        entry
            .files
            .get(index)
            .cloned()
            .ok_or_else(|| ApiError::new("FILE_NOT_FOUND", "Job has no file at this index"))
    }
}
