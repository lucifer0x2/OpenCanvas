//! Runtime enforcement for the exact JSON Schema vocabulary emitted by this server.
use crate::error::{ApiError, Result};
use serde_json::Value;
pub fn validate(schema: &Value, value: &Value) -> Result<()> {
    if let Some(choices) = schema["oneOf"].as_array() {
        if choices
            .iter()
            .filter(|s| validate(s, value).is_ok())
            .count()
            != 1
        {
            return Err(invalid());
        }
    }
    if let Some(expected) = schema.get("const") {
        if expected != value {
            return Err(invalid());
        }
    }
    if let Some(choices) = schema["enum"].as_array() {
        if !choices.contains(value) {
            return Err(invalid());
        }
    }
    if let Some(types) = schema.get("type") {
        let check = |t: &str| match t {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "integer" => value.is_i64() || value.is_u64(),
            "number" => value.is_number(),
            "null" => value.is_null(),
            "boolean" => value.is_boolean(),
            _ => false,
        };
        let valid = types.as_str().map(check).unwrap_or_else(|| {
            types
                .as_array()
                .is_some_and(|a| a.iter().any(|t| t.as_str().is_some_and(check)))
        });
        if !valid {
            return Err(invalid());
        }
    }
    if let Some(n) = value.as_f64() {
        if !n.is_finite()
            || schema["minimum"].as_f64().is_some_and(|min| n < min)
            || schema["maximum"].as_f64().is_some_and(|max| n > max)
            || schema["exclusiveMinimum"]
                .as_f64()
                .is_some_and(|min| n <= min)
        {
            return Err(invalid());
        }
    }
    if let Some(s) = value.as_str() {
        let n = s.chars().count() as u64;
        if schema["minLength"].as_u64().is_some_and(|min| n < min)
            || schema["maxLength"].as_u64().is_some_and(|max| n > max)
        {
            return Err(invalid());
        }
        if let Some(pattern) = schema["pattern"].as_str() {
            let valid = match pattern {
                "^[A-Za-z0-9_-]+$" => {
                    !s.is_empty()
                        && s.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                }
                "^[a-z0-9]+(-[a-z0-9]+)*$" => s.split('-').all(|part| {
                    !part.is_empty()
                        && part
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                }),
                _ => false,
            };
            if !valid {
                return Err(invalid());
            }
        }
    }
    if let Some(array) = value.as_array() {
        let n = array.len() as u64;
        if schema["minItems"].as_u64().is_some_and(|min| n < min)
            || schema["maxItems"].as_u64().is_some_and(|max| n > max)
        {
            return Err(invalid());
        }
        if let Some(items) = schema.get("items") {
            for item in array {
                validate(items, item)?;
            }
        }
    }
    if let Some(object) = value.as_object() {
        if schema["maxProperties"]
            .as_u64()
            .is_some_and(|max| object.len() as u64 > max)
        {
            return Err(invalid());
        }
        if let Some(required) = schema["required"].as_array() {
            for key in required {
                if !object.contains_key(key.as_str().unwrap()) {
                    return Err(invalid());
                }
            }
        }
        for (key, item) in object {
            if let Some(field) = schema["properties"].get(key) {
                validate(field, item)?;
            } else if schema["additionalProperties"] == false {
                return Err(invalid());
            } else if schema["additionalProperties"].is_object() {
                validate(&schema["additionalProperties"], item)?;
            }
        }
    }
    Ok(())
}
fn invalid() -> ApiError {
    ApiError::invalid(
        "Arguments do not match the published tool schema; check required fields, types and limits",
    )
}
