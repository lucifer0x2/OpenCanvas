use crate::{
    cache_hints::Cached,
    domain,
    error::{ApiError, Result},
    jobs::SceneRequest,
    projects, reads, schemas,
    state::AppState,
};
use rmcp::{model::*, service::RequestContext, ErrorData, RoleServer, ServerHandler};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct Mcp(pub AppState);
impl ServerHandler for Mcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new("OpenCanvas", "0.1.0"))
        .with_instructions(crate::instructions::CONNECT)
    }
    async fn list_resources(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> std::result::Result<ListResourcesResult, ErrorData> {
        Ok(crate::resources::list())
    }
    async fn list_resource_templates(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> std::result::Result<ListResourceTemplatesResult, ErrorData> {
        Ok(crate::resources::templates())
    }
    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _: RequestContext<RoleServer>,
    ) -> std::result::Result<ReadResourceResponse, ErrorData> {
        crate::resources::read(&request.uri)
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(schemas::tools()).cached())
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        schemas::tools().into_iter().find(|t| t.name == name)
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResponse, ErrorData> {
        let args = Value::Object(request.arguments.unwrap_or_default());
        let result = dispatch(&self.0, &request.name, args).await;
        Ok(match result {
            Ok(value) => CallToolResult::structured(value),
            Err(error) => CallToolResult::structured_error(error.value()),
        }
        .into())
    }
}
async fn dispatch(state: &AppState, name: &str, args: Value) -> Result<Value> {
    schemas::validate(name, &args)?;
    match name {
        "list_projects" => projects::list(state, serde_json::from_value(args)?, false).await,
        "create_project" => Ok(json!(
            projects::create(state, serde_json::from_value(args)?).await?
        )),
        "update_project" => Ok(json!(
            projects::update(state, serde_json::from_value(args)?).await?
        )),
        "apply_batch" | "apply_batch_file" | "import_asset" => {
            domain::mutate(state, name, args).await
        }
        "get_edit_context" => {
            let snapshot = state
                .snapshot(args["projectId"].as_str().unwrap().into())
                .await?;
            let pages = serde_json::from_value(args["pageIds"].clone())?;
            Ok(opencanvas_storage::edit_context(snapshot, pages)?)
        }
        "get_outline" | "get_subtree" | "get_design_context" | "find_nodes" | "list_tokens"
        | "list_styles" | "list_assets" | "list_fonts" => reads::read(state, name, args).await,
        "get_changes" => reads::changes::read(state, args).await,
        "render_frame" => crate::render_response::render(state, args).await,
        "inspect_layout" => crate::inspect::inspect(state, args).await,
        "export_nodes" => crate::export::export_nodes(state, args).await,
        name if crate::viewer_auth::admin::is_tool(name) => {
            crate::viewer_auth::admin::call(state, name, args).await
        }
        _ => Err(ApiError::new("UNKNOWN_TOOL", "Tool is not available")),
    }
}
pub(crate) async fn scene(
    state: &AppState,
    project: String,
    request: SceneRequest,
) -> Result<Value> {
    let snapshot = state.snapshot(project).await?;
    state.jobs.enqueue(snapshot, request).await
}
pub(crate) async fn render(state: &AppState, mut args: Value) -> Result<Value> {
    let project = args["projectId"]
        .as_str()
        .ok_or_else(|| ApiError::invalid("projectId is required"))?
        .to_owned();
    if let Some(job) = args["jobId"].as_str() {
        let result = state.jobs.get(job).await?;
        if result["projectId"] != project {
            return Err(ApiError::invalid("Job belongs to another project"));
        }
        // Absent or 0 keeps the immediate poll; the schema bounds it to 0–MAX_WAIT_MS.
        return match args["waitMs"].as_u64().filter(|ms| *ms > 0) {
            Some(ms) => {
                let wait = std::time::Duration::from_millis(ms);
                state.jobs.wait(job, wait, state.shutting_down()).await
            }
            None => Ok(result),
        };
    }
    let object = args.as_object_mut().unwrap();
    object.remove("projectId");
    let viewport = object
        .remove("viewport")
        .ok_or_else(|| ApiError::invalid("viewport is required"))?;
    object.insert("width".into(), viewport["width"].clone());
    object.insert("height".into(), viewport["height"].clone());
    let request = serde_json::from_value(args)?;
    scene(state, project, request).await
}
