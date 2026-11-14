//! Directive lines: shared resources (`@token`, `@style`), edits of existing nodes (`@set`,
//! `@del`, `@move`, `@order`) and notes (`notes`). `@in` only changes nesting (see `expand`).
use super::error::Result;
use super::keys;
use super::lexer::Token;
use super::parser::{DirectiveStmt, Part};
use super::props::{self, Target};
use super::{notes, set, value};
use serde_json::{json, Map, Value};

/// Keys a text style may hold (a style with only `fill` is a fill style).
const STYLE_KEYS: [&str; 12] = [
    "fontId",
    "fontFamily",
    "fontWeight",
    "fontStyle",
    "fontSize",
    "lineHeight",
    "fill",
    "letterSpacing",
    "textWrap",
    "textAlign",
    "textDecoration",
    "textTransform",
];

/// The operations of one directive: one each, except `@set` (see `set`).
pub fn operations(d: DirectiveStmt) -> Result<Vec<Value>> {
    let p = &d.positional;
    Ok(vec![match d.name.text.as_str() {
        "token" => token(&d)?,
        "style" => style(&d)?,
        "set" => return set::update(&d),
        "del" => {
            only_positional(&d)?;
            json!({"op":"delete_subtree","id":value::id(&p[0])?})
        }
        "move" => {
            only_positional(&d)?;
            let mut op =
                json!({"op":"reparent_node","id":value::id(&p[0])?,"parentId":value::id(&p[1])?});
            if let Some(index) = p.get(2) {
                op["index"] = value::index(index)?;
            }
            op
        }
        "order" => {
            only_positional(&d)?;
            json!({"op":"reorder_node","id":value::id(&p[0])?,"index":value::index(&p[1])?})
        }
        "note" | "reply" | "note-set" | "resolve" | "note-del" => notes::operation(&d)?,
        other => unreachable!("expand handles @{other}"),
    }])
}

fn token(d: &DirectiveStmt) -> Result<Value> {
    let id = value::id(&d.positional[0])?;
    let raw = &d.positional[1];
    let default = token_value(raw)?;
    let (mut name, mut kind, mut themes) = (json!(id), None, Map::new());
    for part in &d.parts {
        match part {
            Part::Attr(k, v) if k.text == "name" => name = value::literal(v)?,
            Part::Attr(k, v) if k.text == "type" => match v.text.as_str() {
                "color" | "dimension" | "number" => kind = Some(v.text.clone()),
                _ => return Err(v.error("type is color, dimension or number")),
            },
            Part::Attr(theme, v) => {
                themes.insert(theme.text.clone(), token_value(v)?);
            }
            other => return Err(token_of(other).error("@token takes theme=value, type= and name=")),
        }
    }
    let kind = match (kind, &default) {
        (Some(kind), _) => kind,
        (None, Value::String(s)) if s.starts_with('#') => "color".into(),
        (None, Value::Number(_)) => "dimension".into(),
        (None, _) => {
            return Err(raw.error("Add type=color|dimension|number; only #hex and numbers infer it"))
        }
    };
    let value = if themes.is_empty() {
        default
    } else {
        json!({"default": default, "themes": themes})
    };
    Ok(json!({"op":"upsert_token","token":{"id":id,"name":name,"type":kind,"value":value}}))
}

/// `$other` is an alias here, not a `{tokenId}` paint reference.
fn token_value(raw: &Token) -> Result<Value> {
    match raw.text.strip_prefix('$') {
        Some(_) => Ok(json!({"aliasId": value::id(&raw.slice(1, None))?})),
        None => value::scalar(raw),
    }
}

fn style(d: &DirectiveStmt) -> Result<Value> {
    let id = value::id(&d.positional[0])?;
    let (mut name, mut parts) = (json!(id), Vec::new());
    let not_style =
        |at: &Token, key: &str| at.error(format!("`{key}` is not a fill or text style property"));
    for part in &d.parts {
        match part {
            Part::Attr(k, v) if k.text == "name" => name = value::literal(v)?,
            Part::Attr(k, v) if k.text == "props" => {
                // Raw keys are checked here, since `build` merges them unread.
                if let Value::Object(object) = value::scalar(v)? {
                    if let Some(key) = object.keys().find(|k| !STYLE_KEYS.contains(&k.as_str())) {
                        return Err(not_style(v, key));
                    }
                }
                parts.push(part.clone());
            }
            Part::Attr(k, _) if k.text.contains('.') => return Err(not_style(k, &k.text)),
            Part::Attr(k, _) => match keys::effective_key(&k.text) {
                Some(key) if !STYLE_KEYS.contains(&key) => return Err(not_style(k, &k.text)),
                // Unknown keys fail in `build` with a suggestion.
                _ => parts.push(part.clone()),
            },
            other => return Err(token_of(other).error("@style takes key=value attributes")),
        }
    }
    if parts.is_empty() {
        return Err(d.name.error("@style needs fill or text properties"));
    }
    let value = props::build(&parts, Target::Update, None)?;
    let (kind, value) = match value.get("fill") {
        Some(fill) if value.len() == 1 => ("fill", fill.clone()),
        _ => ("text", Value::Object(value)),
    };
    Ok(json!({"op":"upsert_style","style":{"id":id,"name":name,"kind":kind,"value":value}}))
}

pub fn only_positional(d: &DirectiveStmt) -> Result<()> {
    match d.parts.first() {
        Some(part) => Err(token_of(part).error(format!("@{} takes only IDs", d.name.text))),
        None => Ok(()),
    }
}

pub fn token_of(part: &Part) -> &Token {
    match part {
        Part::Style(t) | Part::Flag(t) | Part::Text(t) | Part::Size(t, _) | Part::Attr(t, _) => t,
    }
}
