use crate::{
    cache_hints::Cached,
    domain,
    error::{ApiError, Result},
    jobs::SceneRequest,
    reads, schemas,
    state::AppState,
};
use rmcp::{model::*, service::RequestContext, ErrorData, RoleServer, ServerHandler};
use serde::Deserialize;
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Create {
    pub slug: String,
    pub name: String,
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct List {
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}
pub(crate) async fn projects(state: &AppState, args: Value) -> Result<Value> {
    let req: List = serde_json::from_value(args)?;
    let limit = req.limit.unwrap_or(100);
    if limit == 0 || limit > 100 {
        return Err(ApiError::invalid("Project limit must be 1–100"));
    }
    let all = state.db(|s| s.list_projects()).await?;
    let after: Vec<_> = all
        .into_iter()
        .filter(|p| req.cursor.as_ref().is_none_or(|c| p.slug > *c))
        .collect();
    let next = if after.len() > limit {
        Some(after[limit - 1].slug.clone())
    } else {
        None
    };
    Ok(json!({"projects":after.into_iter().take(limit).collect::<Vec<_>>(),"nextCursor":next}))
}
pub(crate) async fn create(state: &AppState, args: Value) -> Result<Value> {
    let req: Create = serde_json::from_value(args)?;
    let s = state
        .db(move |s| s.create_project(&req.slug, &req.name)?.snapshot())
        .await?;
    let created = json!({"projectId":s.project_id,"slug":s.slug,"name":s.name,"revision":s.revision,"schemaVersion":s.schema_version});
    // A non-durable hint for open viewers; the project list stays the source of truth.
    let _ = state
        .events
        .send(crate::workspace_events::project_created(&created));
    Ok(created)
}
async fn dispatch(state: &AppState, name: &str, args: Value) -> Result<Value> {
    schemas::validate(name, &args)?;
    match name {
        "list_projects" => projects(state, args).await,
        "create_project" => create(state, args).await,
        "apply_batch" | "import_asset" => domain::mutate(state, name, args).await,
        "get_outline" | "get_subtree" | "get_design_context" | "find_nodes" | "list_tokens"
        | "list_styles" | "list_assets" | "list_fonts" => reads::read(state, name, args).await,
        "get_changes" => reads::changes::read(state, args).await,
        "render_frame" => crate::render_response::render(state, args).await,
        "inspect_layout" => crate::inspect::inspect(state, args).await,
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
