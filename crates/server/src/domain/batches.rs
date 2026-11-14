//! File and inline mutation requests converge before candidate validation/commit.
use super::*;
use opencanvas_storage::{BatchFileRequest, BatchRequest};

pub(super) async fn run(state: &AppState, name: &str, mut args: Value) -> Result<(Value, bool)> {
    // The public schema checked this boolean; remove it before decoding the legacy
    // BatchRequest so Rust callers can keep constructing that struct unchanged.
    let dry_run = args.as_object_mut().unwrap().remove("dryRun") == Some(json!(true));
    let page_ids = args.as_object_mut().unwrap().remove("pageIds");
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
    // OCD source compiles to the same operations (ahead of any JSON operations sent with it),
    // then follows the inline path unchanged; storage errors naming an operation also get its
    // source line and column, and results list generated IDs.
    let mut positions = None;
    if let Some(source) = args.as_object_mut().unwrap().remove("source") {
        let source = source.as_str().unwrap_or_default();
        positions = Some(crate::ocd::compile_into(
            &mut args,
            source,
            page_ids.as_ref(),
        )?);
    }
    let locate = |error: crate::ApiError| match &positions {
        Some(positions) => crate::ocd::Positions::locate(positions, error),
        None => error,
    };
    let report = |result: &mut Value| {
        if let Some(positions) = &positions {
            positions.report(result);
        }
    };
    let notes_only = opencanvas_storage::is_note_batch(&args["operations"]);
    if let Some(pages) = &page_ids {
        args["pageIds"] = pages.clone();
    }
    if page_ids.is_some() {
        let request: opencanvas_storage::ScopedBatchRequest = serde_json::from_value(args)?;
        let (project, expected, pages) = (
            request.project_id.clone(),
            request.expected_revision,
            request.page_ids.clone(),
        );
        let result = state
            .project(project.clone(), move |p| {
                if dry_run {
                    serde_json::to_value(p.dry_run_scoped_batch(request)?).map_err(Into::into)
                } else {
                    serde_json::to_value(p.apply_scoped_batch(request)?).map_err(Into::into)
                }
            })
            .await
            .map_err(locate);
        let mut result = conflict::checked(state, &project, expected, result).await?;
        let revision = if dry_run {
            result["baseRevision"].clone()
        } else {
            result["committedRevision"].clone()
        };
        result["editGuard"] =
            json!({"projectId":project,"expectedRevision":revision,"pageIds":pages});
        report(&mut result);
        return Ok((result, notes_only));
    }
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
        .await
        .map_err(locate);
    let mut result = conflict::checked(state, &project, expected, result).await?;
    report(&mut result);
    Ok((result, notes_only))
}
