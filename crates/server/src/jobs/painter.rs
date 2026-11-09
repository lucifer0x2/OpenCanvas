//! The only PNG dispatch seam. Native CPU paint/encode owns the compute lane until drained.
use super::{native, png::Painted, resource_bytes::ResourceBytes};
use crate::{
    error::{ApiError, Result},
    worker::Worker,
};
use opencanvas_render::{encode_png, render, PaintScene, RenderOptions};
use serde_json::{json, Value};
use std::{sync::Arc, time::Instant};

pub(super) enum Painter<'a> {
    Chromium(&'a mut Worker),
    Native {
        bytes: Arc<ResourceBytes>,
        deadline: Instant,
        readiness: Vec<Value>,
    },
}

impl Painter<'_> {
    pub async fn paint(&mut self, scene: &Value, scale: f64) -> Result<Painted> {
        match self {
            Self::Chromium(worker) => {
                let value = worker
                    .call(json!({"op":"render","scene":scene,"scale":scale,"strict":true}))
                    .await?;
                let bytes = super::png::decode(&value)?;
                Ok(Painted { value, bytes })
            }
            Self::Native {
                bytes,
                deadline,
                readiness,
            } => {
                let (bytes, scene) = (bytes.clone(), scene.clone());
                let (result, observed) = native::blocking(*deadline, move |control| {
                    let scene: PaintScene = serde_json::from_value(scene)?;
                    let cancelled = || control.check().is_err();
                    let options = RenderOptions {
                        scale,
                        cancelled: Some(&cancelled),
                        ..Default::default()
                    };
                    let raster = match render(&scene, &bytes.paint(), &options) {
                        Ok(raster) => raster,
                        Err(error) => {
                            control.check()?; // CANCELLED distinguishes timeout from caller cancellation.
                            return Ok((
                                Err(ApiError::new(&error.code, &error.message)),
                                error.readiness,
                            ));
                        }
                    };
                    control.check()?;
                    let png =
                        encode_png(&raster).map_err(|e| ApiError::new(&e.code, &e.message))?;
                    control.check()?;
                    let mut provenance = serde_json::to_value(&scene)?["provenance"].clone();
                    provenance["scale"] = json!(scale);
                    let value = json!({"status":"ready","width":raster.width,"height":raster.height,
                        "draw":raster.draw,"provenance":provenance,
                        "readiness":raster.readiness,"diagnostics":raster.diagnostics});
                    control.check()?;
                    Ok((Ok(Painted { value, bytes: png }), raster.readiness))
                })
                .await?;
                // Missing upstream resources carry a more specific storage observation.
                merge(readiness, observed)?;
                result
            }
        }
    }

    pub async fn inspect_resources(&mut self, scene: &Value) -> Result<()> {
        let Self::Native {
            bytes,
            deadline,
            readiness,
        } = self
        else {
            return Ok(());
        };
        let (bytes, scene) = (bytes.clone(), scene.clone());
        let (result, observed) = native::blocking(*deadline, move |control| {
            let scene: PaintScene = serde_json::from_value(scene)?;
            let cancelled = || control.check().is_err();
            let options = RenderOptions {
                cancelled: Some(&cancelled),
                ..Default::default()
            };
            match opencanvas_render::inspect_svg_resources(&scene, &bytes.paint(), &options) {
                Ok(observed) => Ok((Ok(()), observed)),
                Err(error) => {
                    control.check()?;
                    Ok((
                        Err(ApiError::new(&error.code, &error.message)),
                        error.readiness,
                    ))
                }
            }
        })
        .await?;
        merge(readiness, observed)?;
        result
    }

    pub fn readiness(&self) -> Option<&Vec<Value>> {
        match self {
            Self::Native { readiness, .. } => Some(readiness),
            _ => None,
        }
    }
}

fn merge(
    readiness: &mut Vec<Value>,
    observed: Vec<opencanvas_render::ResourceReadiness>,
) -> Result<()> {
    for observation in observed {
        let value = serde_json::to_value(observation)?;
        if let Some(old) = readiness
            .iter_mut()
            .find(|r| r["id"] == value["id"] && r["kind"] == value["kind"])
        {
            if old["status"] != "failed" {
                *old = value;
            }
        } else {
            readiness.push(value);
        }
    }
    Ok(())
}
