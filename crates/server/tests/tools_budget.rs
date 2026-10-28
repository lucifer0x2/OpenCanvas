//! The one size budget for `tools/list`. Every MCP session pays for the whole catalog before the
//! first call, so it is capped as a total and per tool. Feature tests measure only what their own
//! feature adds (see `notes.rs`), never the whole list, so one tool growing does not break them.
//! Name checks live in `tool_names`; this test also checks `tools/list` is exactly the catalog.
//!
//! Adding or growing a tool: run `cargo test -p opencanvas-server --test tools_budget --
//! --nocapture`, record the printed bytes before and after in the task report, and keep both
//! limits. Raise `TOTAL_MAX` only as a reviewed decision in the same change, by what the new tools
//! need plus the same headroom, and say why in the commit message.
#[allow(dead_code)] // Only `start` and `rpc` are used here.
mod support;
mod tool_names;
use serde_json::{json, Value};
use support::Server;

/// Bytes of the compact `tools` array. 20 955 B with 15 tools at `night/integration` dcfb7ad;
/// the headroom covers the planned notes, export and navigation tools.
const TOTAL_MAX: usize = 32 * 1024;
/// Bytes of any single tool: name, description, annotations and schemas.
const PER_TOOL_MAX: usize = 8 * 1024;

fn size(value: &Value) -> usize {
    serde_json::to_vec(value).unwrap().len()
}

#[tokio::test]
async fn tools_list_is_the_catalog_within_budget() {
    let s = Server::start().await;
    let result = s.rpc("tools/list", json!({})).await;
    let tools = result["tools"].as_array().unwrap();
    tool_names::assert_listed(tools);
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, opencanvas_server::catalog_tool_names());
    let mut sizes: Vec<(usize, &str)> = tools.iter().map(size).zip(names).collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    let total = size(&result["tools"]);
    println!(
        "tools/list: {} tools, {total} B; per tool {sizes:?}",
        tools.len()
    );
    assert!(
        total <= TOTAL_MAX,
        "tools/list is {total} B > {TOTAL_MAX}: {sizes:?}"
    );
    for (bytes, name) in sizes {
        assert!(
            bytes <= PER_TOOL_MAX,
            "{name} is {bytes} B > {PER_TOOL_MAX}"
        );
    }
}
