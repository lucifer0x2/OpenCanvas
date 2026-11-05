use crate::{
    error::{ApiError, Result},
    jobs::SceneRequest,
};
use opencanvas_scene_input::SceneOptions;
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
    let mut input = opencanvas_scene_input::make_input(snapshot, &options)?;
    for font in input["fonts"].as_array_mut().unwrap() {
        font["url"] = format!(
            "{base}/api/v1/projects/{}/fonts/{}/download",
            snapshot.project_id,
            font["id"].as_str().unwrap()
        )
        .into();
    }
    for asset in input["assets"].as_array_mut().unwrap() {
        asset["url"] = format!(
            "{base}/api/v1/projects/{}/assets/{}",
            snapshot.project_id,
            asset["id"].as_str().unwrap()
        )
        .into();
    }
    Ok(input)
}
/// Rejects a request outside the render budget or the supported interaction state.
pub fn check_request(snapshot: &Snapshot, request: &SceneRequest) -> Result<()> {
    if snapshot.revision != request.revision {
        return Err(ApiError::new(
            "REVISION_UNAVAILABLE",
            "Requested authored revision is unavailable",
        ));
    }
    if !request.width.is_finite()
        || !request.height.is_finite()
        || request.width <= 0.0
        || request.height <= 0.0
        || request.width > 100000.0
        || request.height > 100000.0
        || !request.scale.is_finite()
        || request.scale <= 0.0
        || request.scale > 4.0
        || request.width * request.height * request.scale * request.scale > 16_000_000.0
    {
        return Err(ApiError::invalid(
            "Viewport or scale exceeds the render budget",
        ));
    }
    if !matches!(
        request.state.as_str(),
        "default" | "hover" | "pressed" | "focus" | "disabled"
    ) {
        return Err(ApiError::invalid("Unknown interaction state"));
    }
    Ok(())
}
