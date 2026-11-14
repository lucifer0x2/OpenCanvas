//! Note directives, one per note operation: `@note nodeId "text" [id=] [author=]` creates,
//! `@reply noteId "text" [author=]` appends a reply, `@note-set noteId ["text"]
//! [resolved=true|false] [node=nodeId]` edits, `@resolve noteId` closes and `@note-del noteId`
//! deletes. The quoted text may come before or after the attributes.
use super::auto_id::{cut, hash};
use super::directives::{only_positional, token_of};
use super::error::Result;
use super::lexer::Token;
use super::parser::{DirectiveStmt, Part};
use super::value;
use serde_json::{json, Map, Value};

pub fn operation(d: &DirectiveStmt) -> Result<Value> {
    let target = value::id(&d.positional[0])?;
    Ok(match d.name.text.as_str() {
        "note" => create(d, target)?,
        "reply" => {
            let (text, attrs) = read(d, &["author"])?;
            let text = text.ok_or_else(|| d.name.error("@reply needs a quoted text"))?;
            let mut op = json!({"op":"reply_note","id":target,"text":text});
            op.as_object_mut().unwrap().extend(attrs);
            op
        }
        "note-set" => {
            let (text, attrs) = read(d, &["resolved", "node"])?;
            let mut op = json!({"op":"update_note","id":target});
            let fields = op.as_object_mut().unwrap();
            fields.extend(text.map(|text| ("text".to_owned(), text)));
            for (key, value) in attrs {
                let key = if key == "node" { "nodeId".into() } else { key };
                fields.insert(key, value);
            }
            if fields.len() == 2 {
                return Err(d.name.error("@note-set needs \"text\", resolved= or node="));
            }
            op
        }
        "resolve" => {
            only_positional(d)?;
            json!({"op":"update_note","id":target,"resolved":true})
        }
        "note-del" => {
            only_positional(d)?;
            json!({"op":"delete_note","id":target})
        }
        other => unreachable!("directives handles @{other}"),
    })
}

fn create(d: &DirectiveStmt, node: String) -> Result<Value> {
    let (text, mut attrs) = read(d, &["id", "author"])?;
    let text = text.ok_or_else(|| d.name.error("@note needs a quoted text"))?;
    let id = match attrs.remove("id") {
        Some(id) => id,
        None => json!(note_id(&node, text.as_str().unwrap_or_default())),
    };
    let mut op = json!({"op":"create_note","id":id,"nodeId":node,"text":text});
    op.as_object_mut().unwrap().extend(attrs);
    Ok(op)
}

/// The directive's quoted text and its `allowed` attributes: `id`/`node` are IDs,
/// `resolved` a boolean, `author` text.
fn read(d: &DirectiveStmt, allowed: &[&str]) -> Result<(Option<Value>, Map<String, Value>)> {
    let (mut text, mut attrs) = (None, Map::new());
    for part in &d.parts {
        match part {
            Part::Text(t) => text = Some(value::scalar(t)?),
            Part::Attr(k, v) if allowed.contains(&k.text.as_str()) => {
                attrs.insert(k.text.clone(), attribute(&k.text, v)?);
            }
            other => {
                let keys: Vec<String> = allowed.iter().map(|k| format!("{k}=")).collect();
                let message = format!("@{} takes \"text\" and {}", d.name.text, keys.join(", "));
                return Err(token_of(other).error(message));
            }
        }
    }
    Ok((text, attrs))
}

fn attribute(key: &str, raw: &Token) -> Result<Value> {
    match key {
        "id" | "node" => Ok(json!(value::id(raw)?)),
        "resolved" => match value::scalar(raw)? {
            flag @ Value::Bool(_) => Ok(flag),
            _ => Err(raw.error("resolved takes true or false")),
        },
        _ => value::literal(raw),
    }
}

/// `note-<nodeId>-<hash of nodeId and text>`: the same note gets the same ID in any batch,
/// different notes on one node differ, and a long node ID is cut so the ID fits 128 chars.
pub fn note_id(node: &str, text: &str) -> String {
    let digest = hash(&[node, text]) >> 16;
    format!(
        "note-{}-{digest:012x}",
        cut(node, 128 - "note--".len() - 12)
    )
}
