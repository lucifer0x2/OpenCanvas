use super::{id, node_id, object, text};
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
        json!({"id":node_id(),"kind":{"enum":["page","frame","group","rectangle","text","image","instance"]},"parentId":{"type":["string","null"],"maxLength":1160},"props":props,"index":index}),
        &["id", "kind", "parentId", "props"],
    );
    add(
        "update_node",
        json!({"id":node_id(),"props":props}),
        &["id", "props"],
    );
    add(
        "reparent_node",
        json!({"id":node_id(),"parentId":node_id(),"index":index}),
        &["id", "parentId"],
    );
    add(
        "reorder_node",
        json!({"id":node_id(),"index":index}),
        &["id", "index"],
    );
    add("delete_subtree", json!({"id":node_id()}), &["id"]);
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
    let note = text(2000);
    let author = text(64);
    add(
        "create_note",
        json!({"id":id(),"nodeId":id(),"text":note,"author":author}),
        &["id", "nodeId", "text"],
    );
    add(
        "reply_note",
        json!({"id":id(),"text":note,"author":author}),
        &["id", "text"],
    );
    add(
        "update_note",
        json!({"id":id(),"text":note,"resolved":{"type":"boolean"},"nodeId":id()}),
        &["id"],
    );
    add("delete_note", json!({"id":id()}), &["id"]);
    json!({"oneOf":variants})
}
fn token_value() -> Value {
    let branch = json!({"oneOf":[{"type":"number"},{"type":"string","maxLength":16},object(json!({"aliasId":id()}),&["aliasId"])]});
    json!({"oneOf":[branch.clone(),object(json!({"default":branch.clone(),"themes":{"type":"object","maxProperties":32,"additionalProperties":branch}}),&["default","themes"])]})
}
