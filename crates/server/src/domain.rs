use crate::{
    error::{ApiError, Result},
    jobs::SceneRequest,
    reads::changes::conflict,
    state::AppState,
};
use serde_json::{json, Value};
mod batches;
mod import;

pub async fn mutate(state: &AppState, name: &str, args: Value) -> Result<Value> {
    // A commit that only touches notes changes no rendered pixel: skip its prerender.
    let mut notes_only = opencanvas_storage::is_note_batch(&args["operations"]);
    let result = match name {
        "apply_batch" | "apply_batch_file" => {
            let (result, only_notes) = batches::run(state, name, args).await?;
            notes_only = only_notes;
            result
        }
        "import_asset" => import::run(state, args).await?,
        _ => return Err(ApiError::invalid("Unknown mutation")),
    };
    let committed = &result;
    if committed["replayed"] != true && committed["dryRun"] != true {
        let event = json!({"event":"commit","projectId":committed["projectId"],"revision":committed["committedRevision"],"changedIds":committed["changedIds"]});
        let _ = state.events.send(event);
        if let Some(project) = committed["projectId"].as_str().filter(|_| !notes_only) {
            let project = project.to_owned();
            if let Ok(shared) = state.db(move |store| store.shared_snapshot(&project)).await {
                let snapshot = &shared.snapshot;
                // Frames holding changed nodes; token/style-only changes keep the first four.
                let changed: Vec<&str> = committed["changedIds"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                for frame in crate::reads::targets::prerender_shared(&shared, &changed, 4)
                    .unwrap_or_default()
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
                    let _ = state.jobs.prerender_shared(&shared, request).await;
                }
            }
        }
    }
    Ok(result)
}
