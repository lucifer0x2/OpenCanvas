use crate::{
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
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("OpenCanvas", "0.1.0"))
            .with_instructions(crate::instructions::CONNECT)
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: schemas::tools(),
            ..Default::default()
        })
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
    Ok(
        json!({"projectId":s.project_id,"slug":s.slug,"name":s.name,"revision":s.revision,"schemaVersion":s.schema_version}),
    )
}
async fn dispatch(state: &AppState, name: &str, args: Value) -> Result<Value> {
    schemas::validate(name, &args)?;
    match name {
        "list_projects" => projects(state, args).await,
        "create_project" => create(state, args).await,
        "apply_batch" | "import_asset" => domain::mutate(state, name, args).await,
        "get_outline" | "get_subtree" | "get_design_context" | "list_tokens" | "list_styles"
        | "list_assets" | "list_fonts" => reads::read(state, name, args).await,
        "render_frame" => render(state, args).await,
        "inspect_layout" => inspect(state, args).await,
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
async fn render(state: &AppState, mut args: Value) -> Result<Value> {
    let project = args["projectId"]
        .as_str()
        .ok_or_else(|| ApiError::invalid("projectId is required"))?
        .to_owned();
    if let Some(job) = args["jobId"].as_str() {
        let result = state.jobs.get(job).await?;
        if result["projectId"] != project {
            return Err(ApiError::invalid("Job belongs to another project"));
        }
        return Ok(result);
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
async fn inspect(state: &AppState, args: Value) -> Result<Value> {
    let id = args["jobId"]
        .as_str()
        .ok_or_else(|| ApiError::invalid("jobId is required"))?;
    let job = state.jobs.get(id).await?;
    if job["projectId"] != args["projectId"] || job["sceneKey"] != args["sceneKey"] {
        return Err(ApiError::invalid("Job/scene scope mismatch"));
    }
    let ids = args["nodeIds"].as_array();
    if ids.is_some_and(|ids| ids.len() > 200) {
        return Err(ApiError::invalid("At most 200 node IDs"));
    }
    let selected: Vec<_> = job["scene"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|n| ids.is_none_or(|ids| ids.contains(&n["id"])))
        .collect();
    let truncated = selected.len() > 200;
    let nodes: Vec<_> = selected
        .into_iter()
        .take(200)
        .map(|n| json!({"id":n["id"],"bounds":n["bounds"],"lines":n["lines"]}))
        .collect();
    Ok(
        json!({"schemaVersion":1,"projectId":job["projectId"],"revision":job["revision"],"sceneKey":job["sceneKey"],"status":job["status"],"nodes":nodes,"diagnostics":job["diagnostics"],"truncated":truncated}),
    )
}
