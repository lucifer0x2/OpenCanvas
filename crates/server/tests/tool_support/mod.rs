//! One check for "the server publishes its tools", shared by every test that reads `tools/list`.
//! It compares against the catalog (`opencanvas_server::tool_names`) and a floor of tools that
//! must never disappear, so adding a tool never edits a count anywhere.
use serde_json::Value;

/// Tools every build keeps publishing. Append a tool here only to protect it from removal;
/// a new tool needs no change here for the tests to pass.
pub const REQUIRED: [&str; 15] = [
    "list_projects",
    "create_project",
    "get_outline",
    "find_nodes",
    "get_subtree",
    "get_design_context",
    "get_changes",
    "list_tokens",
    "list_styles",
    "list_assets",
    "list_fonts",
    "import_asset",
    "apply_batch",
    "render_frame",
    "inspect_layout",
];

/// Asserts a `tools/list` result publishes exactly the catalog, without duplicates, including
/// every required tool, and returns the published tools.
pub fn assert_catalog(result: &Value) -> &Vec<Value> {
    let tools = result["tools"]
        .as_array()
        .expect("tools/list result has tools");
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        opencanvas_server::tool_names(),
        "tools/list is not the catalog"
    );
    let mut unique = names.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "duplicate tool names: {names:?}");
    for name in REQUIRED {
        assert!(names.contains(&name), "{name} is no longer published");
    }
    tools
}
