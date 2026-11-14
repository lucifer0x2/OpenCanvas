use crate::{
    error::{ApiError, Result},
    jobs::SceneRequest,
};
use opencanvas_scene_input::{check_viewport, SceneOptions};
use opencanvas_storage::Snapshot;
use serde_json::Value;

pub fn make_input(snapshot: &Snapshot, request: &SceneRequest, base: &str) -> Result<Value> {
    let options = SceneOptions {
        frame_id: request.frame_id.clone(),
        width: request.width,
        height: request.height,
        theme: request.theme.clone(),
        state: request.state.clone(),
    };
    let mut input =
        opencanvas_scene_input::make_input_with_scale(snapshot, &options, request.scale)?;
    attach_urls(&mut input, &snapshot.project_id, base);
    Ok(input)
}

pub(super) fn attach_urls(input: &mut Value, project: &str, base: &str) {
    for font in input["fonts"].as_array_mut().unwrap() {
        font["url"] = format!(
            "{base}/api/v1/projects/{}/fonts/{}/download",
            project,
            font["id"].as_str().unwrap()
        )
        .into();
    }
    for asset in input["assets"].as_array_mut().unwrap() {
        asset["url"] = format!(
            "{base}/api/v1/projects/{}/assets/{}",
            project,
            asset["id"].as_str().unwrap()
        )
        .into();
    }
}
/// Rejects a request outside the render budget or the supported interaction state.
pub fn check_request(snapshot: &Snapshot, request: &SceneRequest) -> Result<()> {
    if snapshot.revision != request.revision {
        return Err(ApiError::new(
            "REVISION_UNAVAILABLE",
            "Requested authored revision is unavailable",
        ));
    }
    check_viewport(request.width, request.height, request.scale)?;
    if !matches!(
        request.state.as_str(),
        "default" | "hover" | "pressed" | "focus" | "disabled"
    ) {
        return Err(ApiError::invalid("Unknown interaction state"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
