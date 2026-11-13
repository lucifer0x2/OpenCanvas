//! Shorthands and full catalog keys into authored node props. Every shorthand writes a key
//! of `props_catalog.rs` (names in `keys`); unknown keys fail with a "did you mean"
//! suggestion. Instance descendant props (`ph.r=8`) go through the same mapping.
use super::error::Result;
use super::keys::{effective_key, is_literal, layout_field, unknown};
use super::lexer::Token;
use super::parser::Part;
use super::value;
use opencanvas_storage::instances::INSTANCE_KEYS;
use serde_json::{json, Map, Value};

/// Where the parts are written: a new node, a `use` instance, or an `@set`/`@style` update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Node,
    Instance,
    Update,
}

/// Builds props from parts in order (later wins); `props={json}` is merged last.
/// `text_key` is where a trailing quoted string goes, if the target takes one.
pub fn build(parts: &[Part], target: Target, text_key: Option<&str>) -> Result<Map<String, Value>> {
    let mut props = Map::new();
    let mut merge = None;
    let mut styled = false;
    for part in parts {
        match part {
            Part::Style(token) => {
                if std::mem::replace(&mut styled, true) {
                    return Err(token.error("A node takes one .style"));
                }
                props.insert("styleId".into(), json!(style_id(token)?));
            }
            Part::Size(w, h) => {
                props.insert("width".into(), value::scalar(w)?);
                props.insert("height".into(), value::scalar(h)?);
            }
            Part::Flag(token) => {
                let (key, value) = match token.text.as_str() {
                    "wrap" => ("wrap", json!(true)),
                    "col" => ("mode", json!("column")),
                    mode => ("mode", json!(mode)),
                };
                layout(&mut props, token)?.insert(key.into(), value);
            }
            Part::Attr(key, value) if key.text == "props" => match value::scalar(value)? {
                Value::Object(object) => merge = Some(object),
                _ => return Err(value.error("props takes a {json} object")),
            },
            Part::Attr(key, value) => attr(&mut props, key, value, target)?,
            Part::Text(token) => match text_key {
                Some(key) => {
                    props.insert(key.into(), value::scalar(token)?);
                }
                None => return Err(token.error("A quoted text here needs a key: text=\"…\"")),
            },
        }
    }
    props.extend(merge.unwrap_or_default());
    Ok(props)
}

/// `.styleId`: any ID `@style` accepts (`.card`, `.123`, `.-ink`).
fn style_id(token: &Token) -> Result<String> {
    let id = token.slice(1, None);
    if id.text.is_empty() {
        return Err(token.error("A .style needs an ID after the dot, e.g. .card"));
    }
    value::id(&id)
}

fn attr(props: &mut Map<String, Value>, key: &Token, raw: &Token, target: Target) -> Result<()> {
    let k = key.text.as_str();
    if let Some((descendant, _)) = k.split_once('.') {
        if target != Target::Instance {
            return Err(unknown(key));
        }
        return descendant_prop(props, key, descendant, raw);
    }
    let effective = effective_key(k);
    if target == Target::Instance && !effective.is_some_and(|e| INSTANCE_KEYS.contains(&e)) {
        return descendant_value(props, key, raw);
    }
    let effective = effective.ok_or_else(|| unknown(key))?;
    if let Some(field) = layout_field(k) {
        let mut value = value::parse(raw)?;
        if field == "justify" && value == "between" {
            value = json!("space-between");
        }
        layout(props, key)?.insert(field.into(), value);
    } else if k == "size" {
        let parts = value::split(raw, '/');
        if parts.len() > 2 {
            return Err(raw.error("size takes fontSize or fontSize/lineHeight"));
        }
        props.insert("fontSize".into(), value::scalar(&parts[0])?);
        if let Some(line) = parts.get(1) {
            props.insert("lineHeight".into(), value::scalar(line)?);
        }
    } else if k == "stroke" {
        props.insert("stroke".into(), stroke(raw)?);
    } else if k == "overrides" {
        match value::scalar(raw)? {
            Value::Object(object) => props
                .entry("overrides")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| raw.error("overrides must be an object"))?
                .extend(object),
            _ => return Err(raw.error("overrides takes a {json} object")),
        }
    } else {
        let value = match is_literal(effective) {
            true => value::literal(raw)?,
            false => value::parse(raw)?,
        };
        props.insert(effective.into(), value);
    }
    Ok(())
}

/// `ph.<key>=value` on `use`: `<key>` is read like a node prop, shorthands included, into
/// the override of master descendant `ph`.
fn descendant_prop(
    props: &mut Map<String, Value>,
    key: &Token,
    descendant: &str,
    raw: &Token,
) -> Result<()> {
    let len = descendant.chars().count();
    let id = value::id(&key.slice(0, Some(len)))?;
    let prop = key.slice(len + 1, None);
    attr(override_entry(props, id), &prop, raw, Target::Node)
}

/// `ph="x"` on `use` sets the descendant's text; `ph={json}` merges a whole override object.
fn descendant_value(props: &mut Map<String, Value>, key: &Token, raw: &Token) -> Result<()> {
    let descendant = value::id(key)?;
    let entry = match value::literal(raw)? {
        Value::Object(object) if raw.text.starts_with('{') => object,
        text @ Value::String(_) => Map::from_iter([("text".into(), text)]),
        _ => return Err(raw.error("A descendant override takes \"text\" or a {json} object")),
    };
    override_entry(props, descendant).extend(entry);
    Ok(())
}

/// `stroke=color[,width[,align]]`, or a raw `{json}` object.
fn stroke(raw: &Token) -> Result<Value> {
    if raw.text.starts_with('{') {
        return value::scalar(raw);
    }
    Ok(match value::parse(raw)? {
        Value::Array(items) if items.len() <= 3 => {
            let mut stroke = Map::new();
            for (key, item) in ["color", "width", "align"].into_iter().zip(items) {
                stroke.insert(key.into(), item);
            }
            Value::Object(stroke)
        }
        Value::Array(_) => return Err(raw.error("stroke takes color[,width[,align]]")),
        color => json!({ "color": color }),
    })
}

fn layout<'a>(props: &'a mut Map<String, Value>, at: &Token) -> Result<&'a mut Map<String, Value>> {
    props
        .entry("layout")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| at.error("layout is not an object here"))
}

fn override_entry(props: &mut Map<String, Value>, descendant: String) -> &mut Map<String, Value> {
    let overrides = props.entry("overrides").or_insert_with(|| json!({}));
    if !overrides.is_object() {
        *overrides = json!({});
    }
    let entry = overrides
        .as_object_mut()
        .unwrap()
        .entry(descendant)
        .or_insert_with(|| json!({}));
    if !entry.is_object() {
        *entry = json!({});
    }
    entry.as_object_mut().unwrap()
}
