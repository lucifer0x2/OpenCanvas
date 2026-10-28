use super::{id, object, text};
use serde_json::{json, Value};
pub fn schema() -> Value {
    let mut variants = Vec::new();
    let mut add = |op: &str, mut props: Value, required: &[&str]| {
        props["op"] = json!({"const":op});
        let mut required = required.to_vec();
        required.push("op");
        variants.push(object(props, &required));
    };
    let props = json!({"type":"object","maxProperties":50});
    let index = json!({"type":"integer","minimum":0,"maximum":20000});
    add(
        "create_node",
        json!({"id":id(),"kind":{"enum":["page","frame","group","rectangle","text","image"]},"parentId":{"type":["string","null"],"maxLength":128},"props":props,"index":index}),
        &["id", "kind", "parentId", "props"],
    );
    add(
        "update_node",
        json!({"id":id(),"props":props}),
        &["id", "props"],
    );
    add(
        "reparent_node",
        json!({"id":id(),"parentId":id(),"index":index}),
        &["id", "parentId"],
    );
    add(
        "reorder_node",
        json!({"id":id(),"index":index}),
        &["id", "index"],
    );
    add("delete_subtree", json!({"id":id()}), &["id"]);
    add(
        "upsert_token",
        json!({"token":object(json!({"id":id(),"name":text(256),"type":{"enum":["color","dimension","number"]},"value":token_value()}),&["id","name","type","value"])}),
        &["token"],
    );
    add(
        "upsert_style",
        json!({"style":object(json!({"id":id(),"name":text(256),"kind":{"enum":["fill","text"]},"value":{"oneOf":[{"type":"string","maxLength":16},{"type":"object","maxProperties":50}]}}),&["id","name","kind","value"])}),
        &["style"],
    );
    json!({"oneOf":variants})
}
fn token_value() -> Value {
    let branch = json!({"oneOf":[{"type":"number"},{"type":"string","maxLength":16},object(json!({"aliasId":id()}),&["aliasId"])]});
    json!({"oneOf":[branch.clone(),object(json!({"default":branch.clone(),"themes":{"type":"object","maxProperties":32,"additionalProperties":branch}}),&["default","themes"])]})
}
