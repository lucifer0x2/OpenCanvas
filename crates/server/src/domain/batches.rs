//! File and inline mutation requests converge before candidate validation/commit.
use super::*;
use opencanvas_storage::{BatchFileRequest, BatchRequest};

pub(super) async fn run(state: &AppState, name: &str, mut args: Value) -> Result<(Value, bool)> {
    // The public schema checked this boolean; remove it before decoding the legacy
    // BatchRequest so Rust callers can keep constructing that struct unchanged.
    let dry_run = args.as_object_mut().unwrap().remove("dryRun") == Some(json!(true));
    if name == "apply_batch_file" {
        let file: BatchFileRequest = serde_json::from_value(args)?;
        let to_read = file.clone();
        let operations = state
            .project(file.project_id.clone(), move |p| {
                p.read_batch_file_operations(&to_read)
            })
            .await?;
        args = json!({"projectId":file.project_id,"expectedRevision":file.expected_revision,
            "idempotencyKey":file.idempotency_key,"operations":operations});
        // Validate original JSON, before deserializing optional operation fields.
        crate::schemas::validate("apply_batch", &args)?;
    }
    let notes_only = opencanvas_storage::is_note_batch(&args["operations"]);
    let request: BatchRequest = serde_json::from_value(args)?;
    let (project, expected) = (request.project_id.clone(), request.expected_revision);
    let result = state
        .project(project.clone(), move |p| {
            if dry_run {
                serde_json::to_value(p.dry_run_batch(request)?).map_err(Into::into)
            } else {
                serde_json::to_value(p.apply_batch(request)?).map_err(Into::into)
            }
        })
        .await;
    Ok((
        conflict::checked(state, &project, expected, result).await?,
        notes_only,
    ))
}
