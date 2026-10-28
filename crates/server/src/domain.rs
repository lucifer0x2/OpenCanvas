use crate::{
    error::{ApiError, Result},
    jobs::SceneRequest,
    state::AppState,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use opencanvas_storage::{BatchRequest, FontRegistration, ImportAssetRequest};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportRequest {
    project_id: String,
    expected_revision: u64,
    idempotency_key: String,
    media_type: String,
    base64: String,
    font: Option<FontRegistration>,
}
pub async fn mutate(state: &AppState, name: &str, args: Value) -> Result<Value> {
    let result = match name {
        "apply_batch" => {
            let req: BatchRequest = serde_json::from_value(args)?;
            let result = state
                .db(move |s| s.open_project(&req.project_id)?.apply_batch(req))
                .await?;
            serde_json::to_value(result)?
        }
        "import_asset" => {
            let req: ImportRequest = serde_json::from_value(args)?;
            if req.base64.len() > 13_981_016 {
                return Err(ApiError::invalid("Asset exceeds 10 MiB"));
            }
            let bytes = STANDARD
                .decode(&req.base64)
                .map_err(|_| ApiError::invalid("Invalid base64 asset"))?;
            if bytes.len() > opencanvas_storage::MAX_ASSET_BYTES {
                return Err(ApiError::invalid("Asset exceeds 10 MiB"));
            }
            let req = ImportAssetRequest {
                project_id: req.project_id,
                expected_revision: req.expected_revision,
                idempotency_key: req.idempotency_key,
                media_type: req.media_type,
                bytes,
                font: req.font,
            };
            serde_json::to_value(
                state
                    .db(move |s| s.open_project(&req.project_id)?.import_asset(req))
                    .await?,
            )?
        }
        _ => return Err(ApiError::invalid("Unknown mutation")),
    };
    let committed = result.get("result").unwrap_or(&result);
    if committed["replayed"] != true {
        let event = json!({"event":"commit","projectId":committed["projectId"],"revision":committed["committedRevision"],"changedIds":committed["changedIds"]});
        let _ = state.events.send(event);
        if let Some(project) = committed["projectId"].as_str() {
            if let Ok(snapshot) = state.snapshot(project.into()).await {
                for frame in snapshot
                    .nodes
                    .iter()
                    .filter(|n| n.kind == opencanvas_storage::NodeKind::Frame)
                    .take(4)
                {
                    let width = frame.props["width"].as_f64().unwrap_or(800.0);
                    let height = frame.props["height"].as_f64().unwrap_or(600.0);
                    let request = SceneRequest {
                        frame_id: frame.id.clone(),
                        revision: snapshot.revision,
                        width,
                        height,
                        theme: "default".into(),
                        state: "default".into(),
                        scale: 1.0,
                    };
                    let _ = state.jobs.enqueue(snapshot.clone(), request).await;
                }
            }
        }
    }
    Ok(result)
}
