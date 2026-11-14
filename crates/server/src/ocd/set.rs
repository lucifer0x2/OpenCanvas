//! `@set id {attr}` compiles to `update_node`. Whole values (`fill=`, `layout={…}`,
//! `overrides={…}`, `stroke=c,w,a`, `props={…}`) replace the stored prop. Partial writes of
//! an object prop merge into the stored object at commit (`merge:true`, a JSON merge patch):
//! layout flags and `p`/`gap`/`align`/`justify`, `<prop>.<field>=` (`layout.gap=8`,
//! `stroke.width=2`), `<descendant>.<key>=` instance overrides, and `stroke=color[,width]`.
//! A statement with both writes a replacing update for object values, then a merging one.
use super::error::Result;
use super::keys::{self, effective_key, layout_field};
use super::lexer::Token;
use super::parser::{DirectiveStmt, Part};
use super::props::{self, Target};
use super::{value, OcdError};
use opencanvas_storage::{merge_props, merges};
use serde_json::{json, Map, Value};

pub fn update(d: &DirectiveStmt) -> Result<Vec<Value>> {
    if d.parts.is_empty() {
        return Err(d.name.error("@set needs at least one attribute"));
    }
    let id = value::id(&d.positional[0])?;
    let (mut whole, mut patch) = (Vec::new(), Map::new());
    for part in &d.parts {
        match partial(part)? {
            Some(fields) => merge_props(&mut patch, &fields),
            None => whole.push(part.clone()),
        }
    }
    let mut replace = props::build(&whole, Target::Update, None)?;
    if patch.is_empty() {
        return Ok(vec![op(&id, replace, false)]);
    }
    // A prop written whole takes this statement's partial writes before it replaces.
    for key in replace.keys().cloned().collect::<Vec<_>>() {
        if let Some(fields) = patch.remove(&key) {
            merge_props(&mut replace, &Map::from_iter([(key, fields)]));
        }
    }
    // Scalars, lists and references replace in a merge patch too; records and `null` do not.
    let (records, values): (Map<_, _>, Map<_, _>) = replace
        .into_iter()
        .partition(|(_, value)| merges(value) || value.is_null());
    patch.extend(values);
    let mut ops = Vec::new();
    if !records.is_empty() {
        ops.push(op(&id, records, false));
    }
    if !patch.is_empty() {
        ops.push(op(&id, patch, true));
    }
    Ok(ops)
}

fn op(id: &str, props: Map<String, Value>, merge: bool) -> Value {
    let mut op = json!({"op":"update_node","id":id,"props":props});
    if merge {
        op["merge"] = json!(true);
    }
    op
}

/// The props one partial write patches, or `None` for a whole value.
fn partial(part: &Part) -> Result<Option<Map<String, Value>>> {
    let single = |target| props::build(std::slice::from_ref(part), target, None).map(Some);
    match part {
        Part::Flag(_) => single(Target::Update),
        Part::Attr(key, raw) if key.text == "stroke" => {
            // A JSON object or all three list fields replace; a color[,width] list patches.
            match raw.text.starts_with('{') || value::split(raw, ',').len() == 3 {
                true => Ok(None),
                false => single(Target::Update),
            }
        }
        Part::Attr(key, raw) => match key.text.split_once('.') {
            Some((prefix, field)) => match effective_key(prefix) {
                Some(prop) => field_patch(key, prefix, prop, field, raw).map(Some),
                // `ph.fill=` overrides instance descendant `ph`, read like a `use` attribute.
                None => single(Target::Instance),
            },
            None if layout_field(&key.text).is_some() => single(Target::Update),
            None if effective_key(&key.text).is_none() && key.text != "props" => Err(unknown(key)),
            None => Ok(None),
        },
        _ => Ok(None),
    }
}

/// `layout.gap=8`, `stroke.width=2`, `layout.padding=8,16`: one field of an object prop.
fn field_patch(
    key: &Token,
    prefix: &str,
    prop: &str,
    field: &str,
    raw: &Token,
) -> Result<Map<String, Value>> {
    let at = key.slice(prefix.chars().count() + 1, None);
    let mut value = value::parse(raw)?;
    for name in field.rsplit('.') {
        let valid = !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric());
        if !valid {
            return Err(at.error(format!("`{field}` is not a field name of `{prop}`")));
        }
        value = json!({ name: value });
    }
    Ok(Map::from_iter([(prop.to_owned(), value)]))
}

/// An unknown key on `@set`; an instance descendant is written `<id>.<key>=`.
fn unknown(token: &Token) -> OcdError {
    let error = keys::unknown(token);
    OcdError {
        message: format!(
            "{}; for an instance descendant write <descendantId>.text=\"…\"",
            error.message
        ),
        ..error
    }
}
