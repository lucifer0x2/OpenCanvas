//! Tools every build must publish. Checks match names instead of counting the list, so adding a
//! tool does not break them; add a name here only when its own tests need it guarded.
use serde_json::Value;
use std::collections::HashSet;

pub const EXPECTED: [&str; 16] = [
    "list_projects",
    "create_project",
    "get_outline",
    "find_nodes",
    "get_subtree",
    "get_design_context",
    "list_tokens",
    "list_styles",
    "list_fonts",
    "list_assets",
    "get_changes",
    "apply_batch",
    "import_asset",
    "render_frame",
    "export_nodes",
    "inspect_layout",
];

/// Asserts `tools/list` names are unique and include every expected tool.
pub fn assert_listed(tools: &[Value]) {
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert_eq!(names.len(), tools.len(), "a tool has no name");
    let unique: HashSet<&str> = names.iter().copied().collect();
    assert_eq!(unique.len(), names.len(), "duplicate tool names: {names:?}");
    let missing: Vec<&str> = EXPECTED
        .iter()
        .copied()
        .filter(|n| !unique.contains(n))
        .collect();
    assert!(missing.is_empty(), "missing tools {missing:?} in {names:?}");
}
