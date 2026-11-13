//! Attribute values: numbers, booleans, `$token` references, quoted JSON strings, inline
//! `{json}`/`[json]`, comma lists, and anything else as a bare string (`fill`, `hug`, `50%`,
//! `#hex`, keywords).
use super::error::Result;
use super::lexer::Token;
use serde_json::{json, Value};

/// Parses one value. A top-level comma makes a list of values.
pub fn parse(token: &Token) -> Result<Value> {
    let parts = split(token, ',');
    if parts.len() > 1 {
        return parts
            .iter()
            .map(scalar)
            .collect::<Result<Vec<_>>>()
            .map(Value::Array);
    }
    scalar(token)
}

/// A value read as a string unless it is quoted, JSON or a `$token`: names and text stay
/// literal even when they look like numbers.
pub fn literal(token: &Token) -> Result<Value> {
    match token.text.chars().next() {
        Some('"' | '{' | '[' | '$') => scalar(token),
        _ => Ok(Value::String(token.text.clone())),
    }
}

pub fn scalar(token: &Token) -> Result<Value> {
    let text = token.text.as_str();
    match text.chars().next() {
        None => Err(token.error("Missing value")),
        Some('"') => serde_json::from_str::<String>(text)
            .map(Value::String)
            .map_err(|e| token.error(format!("Invalid string: {e}"))),
        Some('{' | '[') => {
            serde_json::from_str(text).map_err(|e| token.error(format!("Invalid JSON: {e}")))
        }
        Some('$') => Ok(json!({"tokenId": id(&token.slice(1, None))?})),
        Some(first) if first.is_ascii_digit() || matches!(first, '-' | '.') => {
            Ok(number(text).unwrap_or_else(|| Value::String(text.into())))
        }
        _ => Ok(match text {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => Value::String(text.into()),
        }),
    }
}

/// Integral numbers stay integers so compiled JSON matches hand-written operations.
pub fn number(text: &str) -> Option<Value> {
    let n: f64 = text.parse().ok().filter(|n: &f64| n.is_finite())?;
    if n.fract() == 0.0 && n.abs() < 1e15 {
        Some(json!(n as i64))
    } else {
        Some(json!(n))
    }
}

/// A sibling index (`index=` on a node, the third ID of `@move`): an integer from 0.
pub fn index(raw: &Token) -> Result<Value> {
    match scalar(raw)? {
        Value::Number(n) if n.is_u64() => Ok(Value::Number(n)),
        _ => Err(raw.error("index takes an integer from 0")),
    }
}

/// A resource or node ID: letters, digits, `_` and `-`.
pub fn id(token: &Token) -> Result<String> {
    let valid = !token.text.is_empty()
        && token
            .text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if valid {
        Ok(token.text.clone())
    } else {
        Err(token.error(format!(
            "Invalid ID `{}`; use letters, digits, _ and -",
            token.text
        )))
    }
}

/// Splits on `sep` outside quoted strings and JSON brackets, keeping each part's column.
/// One pass: each part is cut from the scanned characters, so a long list stays linear.
pub fn split(token: &Token, sep: char) -> Vec<Token> {
    let chars: Vec<char> = token.text.chars().collect();
    let (mut parts, mut start, mut depth, mut quoted) = (Vec::new(), 0, 0i32, false);
    let cut = |from: usize, to: usize| token.part(chars[from..to].iter().collect(), from);
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' if quoted => i += 1,
            '"' => quoted = !quoted,
            '{' | '[' if !quoted => depth += 1,
            '}' | ']' if !quoted => depth -= 1,
            c if c == sep && !quoted && depth == 0 => {
                parts.push(cut(start, i));
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(cut(start, chars.len()));
    parts
}
