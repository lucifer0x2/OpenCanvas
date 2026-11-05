use super::{values::*, Candidate};
use crate::{error::*, model::Token};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) const MAX_ALIAS_DEPTH: usize = 32;
const MAX_THEMES: usize = 32;
const MAX_TOKENS: usize = 4096;

/// A token envelope is deliberately shallow: branches cannot contain envelopes.
pub(super) fn branches(token: &Token) -> Result<Vec<(&str, &Value)>> {
    let value = &token.value;
    if !value.is_object() || value.get("aliasId").is_some() {
        return Ok(vec![("default", value)]);
    }
    let object = value.as_object().unwrap();
    if object.len() != 2 || !object.contains_key("default") {
        return Err(StorageError::validation(
            "token.value",
            "Expected a literal, aliasId or default/themes envelope",
        ));
    }
    let themes = object
        .get("themes")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            StorageError::validation(
                "token.themes",
                "Themes must be a bounded name-to-value object",
            )
        })?;
    if themes.len() > MAX_THEMES {
        return Err(StorageError::validation(
            "token.themes",
            "At most 32 named themes are supported",
        ));
    }
    let mut out = vec![("default", &object["default"])];
    for (name, branch) in themes {
        if name == "default" || name.len() > 64 || !valid_id(name) {
            return Err(StorageError::validation("token.themes", "Theme names require 1–64 ASCII letters, digits, underscores or hyphens; default is reserved"));
        }
        out.push((name.as_str(), branch));
    }
    Ok(out)
}

pub(super) fn alias(value: &Value) -> Result<Option<&str>> {
    if !value.is_object() {
        return Ok(None);
    }
    let object = value.as_object().unwrap();
    let target = object
        .get("aliasId")
        .and_then(Value::as_str)
        .filter(|id| valid_id(id));
    if object.len() != 1 || target.is_none() {
        return Err(StorageError::validation(
            "token.value",
            "A token branch must be a literal or exact aliasId reference",
        ));
    }
    Ok(target)
}

impl Candidate {
    pub(super) fn validate_tokens(&self) -> Result<BTreeSet<String>> {
        if self.tokens.len() > MAX_TOKENS {
            return Err(StorageError::validation(
                "tokens",
                "At most 4096 tokens are supported",
            ));
        }
        let mut themes = BTreeSet::from(["default".into()]);
        for token in self.tokens.values() {
            id(&token.id)?;
            if token.name.len() > 256 {
                return Err(StorageError::validation(
                    "token.name",
                    "Name exceeds 256 bytes",
                ));
            }
            if !matches!(token.token_type.as_str(), "color" | "dimension" | "number") {
                return Err(StorageError::validation(
                    "token.type",
                    "Only color, dimension and number tokens are supported",
                ));
            }
            for (theme, value) in branches(token)? {
                themes.insert(theme.into());
                if let Some(target_id) = alias(value)? {
                    let target = self.tokens.get(target_id).ok_or_else(|| {
                        StorageError::validation(
                            "token.aliasId",
                            "Alias target does not exist; forward references are not supported",
                        )
                    })?;
                    if target.token_type != token.token_type {
                        return Err(StorageError::validation(
                            "token.aliasId",
                            "Alias target must have the same token type",
                        ));
                    }
                } else {
                    literal(value, &token.token_type)?;
                }
            }
        }
        if themes.len() > MAX_THEMES + 1 {
            return Err(StorageError::validation(
                "token.themes",
                "At most 32 named themes per project are supported",
            ));
        }
        self.validate_token_graph()?;
        Ok(themes)
    }

    pub(super) fn select_theme(&mut self, theme: &str) -> Result<()> {
        if !self.validate_tokens()?.contains(theme) {
            let mut error = StorageError::new(
                "UNSUPPORTED_THEME",
                "Requested design theme is not declared by any token",
            );
            error.field = Some("theme".into());
            return Err(error);
        }
        self.theme = theme.into();
        Ok(())
    }

    pub(super) fn token_value(&self, id: &str, theme: &str) -> Result<Value> {
        let mut current = id;
        let mut seen = BTreeSet::new();
        for _ in 0..=MAX_ALIAS_DEPTH {
            if !seen.insert(current) {
                return Err(StorageError::validation(
                    "token.aliasId",
                    "Token alias cycle detected",
                ));
            }
            let token = self.tokens.get(current).ok_or_else(|| {
                StorageError::validation("token.aliasId", "Alias target does not exist")
            })?;
            let choices = branches(token)?;
            let value = choices
                .iter()
                .find(|(name, _)| *name == theme)
                .unwrap_or(&choices[0])
                .1;
            if let Some(target) = alias(value)? {
                current = target;
            } else {
                literal(value, &token.token_type)?;
                return Ok(value.clone());
            }
        }
        Err(StorageError::validation(
            "token.aliasId",
            "Token alias depth exceeds 32",
        ))
    }
}
