//! `@set id {attr}` compiles to `update_node`, which merges props only at the top level, so an
//! object-valued prop is replaced whole. Shorthands that assemble part of an object (layout
//! flags and keys, descendant overrides) would silently drop the stored rest of that object;
//! here they fail with a hint to write the object in full (`layout={…}`, `overrides={…}`).
//! `stroke=` is accepted only with color, width and align all given.
use super::error::Result;
use super::keys::{self, effective_key};
use super::lexer::Token;
use super::parser::{DirectiveStmt, Part};
use super::props::{self, Target};
use super::{value, OcdError};
use serde_json::{json, Value};

const LAYOUT_EXAMPLE: &str = r#"layout={"mode":"column","padding":32,"gap":8}"#;
const OVERRIDES_EXAMPLE: &str = r#"overrides={"ph":{"text":"x","fill":{"tokenId":"pri"}}}"#;
const STROKE_EXAMPLE: &str =
    r##"stroke=#E5E7EB,1,inside or stroke={"color":"#E5E7EB","width":1,"align":"inside"}"##;

pub fn update(d: &DirectiveStmt) -> Result<Value> {
    if d.parts.is_empty() {
        return Err(d.name.error("@set needs at least one attribute"));
    }
    for part in &d.parts {
        reject_partial(part)?;
    }
    let props = props::build(&d.parts, Target::Update, None)?;
    Ok(json!({"op":"update_node","id":value::id(&d.positional[0])?,"props":props}))
}

fn reject_partial(part: &Part) -> Result<()> {
    let (token, key) = match part {
        Part::Flag(flag) => return Err(replaced(flag, &flag.text, "layout")),
        Part::Attr(key, raw) if key.text == "stroke" => {
            // A JSON object is explicit; a list must name all three fields.
            let full = raw.text.starts_with('{') || value::split(raw, ',').len() == 3;
            return match full {
                true => Ok(()),
                false => Err(replaced(key, &format!("stroke={}", raw.text), "stroke")),
            };
        }
        Part::Attr(key, _) => (key, key.text.as_str()),
        _ => return Ok(()),
    };
    if let Some((prefix, _)) = key.split_once('.') {
        // `ph.fill=` overrides a descendant; `layout.gap=` would be read the same way.
        let object = effective_key(prefix).unwrap_or("overrides");
        return Err(replaced(token, &format!("{key}="), object));
    }
    match effective_key(key) {
        Some("layout") if key != "layout" => Err(replaced(token, &format!("{key}="), "layout")),
        Some(_) => Ok(()),
        None if key == "props" => Ok(()),
        None => Err(unknown(token)),
    }
}

/// `padding=` suggests `p`, itself a layout shorthand: point at `layout={…}` instead. Any
/// other unknown key may be an instance descendant, which only `overrides={…}` can change.
fn unknown(token: &Token) -> OcdError {
    let error = keys::unknown(token);
    match error.suggestion.as_deref().and_then(effective_key) {
        Some("layout") => replaced(token, &format!("{}=", token.text), "layout"),
        _ => OcdError {
            message: format!(
                "{}; for an instance descendant, @set replaces all overrides: write {OVERRIDES_EXAMPLE} in full",
                error.message
            ),
            ..error
        },
    }
}

fn replaced(token: &Token, written: &str, object: &str) -> OcdError {
    let example = match object {
        "layout" => LAYOUT_EXAMPLE.to_owned(),
        "overrides" => OVERRIDES_EXAMPLE.to_owned(),
        "stroke" => STROKE_EXAMPLE.to_owned(),
        other => format!("{other}={{…}}"),
    };
    token.error(format!(
        "`{written}` would write part of `{object}`, but @set replaces the whole object and drops its stored fields; write it in full, e.g. {example}"
    ))
}
