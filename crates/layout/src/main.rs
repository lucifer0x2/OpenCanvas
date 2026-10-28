use opencanvas_layout::{LayoutInput, MeasureCache, compute};
use serde::Deserialize;
use std::io::{self, BufRead, Write};
#[derive(Deserialize)]
struct Request {
    input: LayoutInput,
    #[serde(default)]
    cache: MeasureCache,
}
fn main() {
    for line in io::stdin().lock().lines() {
        let result=match line { Ok(line)=>serde_json::from_str::<Request>(&line).map_err(|e|serde_json::json!({"status":"failed","code":"INVALID_JSON","message":e.to_string()})).and_then(|r|compute(&r.input,&r.cache).map(|r|serde_json::to_value(r).unwrap()).map_err(|e|serde_json::json!({"status":"failed","code":e.code,"message":e.message}))), Err(e)=>Err(serde_json::json!({"status":"failed","code":"IO_ERROR","message":e.to_string()})) };
        println!("{}", result.unwrap_or_else(|e| e));
        let _ = io::stdout().flush();
    }
}
