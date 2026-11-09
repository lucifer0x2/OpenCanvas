//! Exact imported static-face selection. Never synthesize or choose a nearby weight.
use super::Candidate;
use crate::{Font, Result, StorageError};
use serde_json::{Map, Value};

pub(super) fn check_face_property(key: &str, value: &Value) -> Result<()> {
    let valid = match key {
        "fontFamily" => value
            .as_str()
            .is_some_and(|s| !s.is_empty() && s.len() <= 128),
        "fontWeight" => value
            .as_f64()
            .is_some_and(|n| (100. ..=900.).contains(&n) && n.fract() == 0.),
        "fontStyle" => matches!(value.as_str(), Some("normal" | "italic")),
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err(StorageError::validation(
            key,
            match key {
                "fontFamily" => "Font family must be a nonempty string up to 128 bytes",
                "fontWeight" => "Font weight must be an integer from 100 to 900",
                _ => "Font style must be normal or italic",
            },
        ))
    }
}

impl Candidate {
    pub(super) fn select_font_face(
        &self,
        out: &mut Map<String, Value>,
        authored: &Map<String, Value>,
    ) -> Result<()> {
        if authored.contains_key("fontId") && authored.contains_key("fontFamily") {
            return Err(StorageError::validation(
                "props.fontFamily",
                "Use fontId or fontFamily on a text node, not both",
            ));
        }
        if !["fontFamily", "fontWeight", "fontStyle"]
            .iter()
            .any(|k| out.contains_key(*k))
        {
            return Ok(());
        }
        for key in ["fontFamily", "fontWeight", "fontStyle"] {
            if let Some(value) = out.get(key) {
                check_face_property(key, value)?;
            }
        }
        // An explicit node base replaces the style's base, while scalar overrides merge.
        let direct = authored.contains_key("fontId") || authored.contains_key("fontFamily");
        let base = if direct { authored } else { &*out };
        let font = base
            .get("fontId")
            .map(|id| {
                self.fonts
                    .get(id.as_str().unwrap_or_default())
                    .ok_or_else(|| {
                        StorageError::validation("fontId", "Referenced font does not exist")
                    })
            })
            .transpose()?;
        let family = base
            .get("fontFamily")
            .and_then(Value::as_str)
            .or_else(|| font.map(|f| f.family.as_str()));
        let Some(family) = family else {
            return Ok(());
        };
        let faces: Vec<&Font> = self.fonts.values().filter(|f| f.family == family).collect();
        if faces.is_empty() {
            return Err(self.missing_family(family));
        }
        let weight = out
            .get("fontWeight")
            .and_then(Value::as_f64)
            .map(|n| n as u16)
            .unwrap_or_else(|| font.map_or(400, |f| f.weight));
        let style = out
            .get("fontStyle")
            .and_then(Value::as_str)
            .unwrap_or_else(|| font.map_or("normal", |f| f.style.as_str()));
        let matches: Vec<_> = faces
            .iter()
            .filter(|f| f.weight == weight && f.style == style)
            .copied()
            .collect();
        let selected = match matches.as_slice() {
            [one] => *one,
            [] => {
                let available = faces
                    .iter()
                    .map(|f| format!("{} {}", f.weight, f.style))
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ");
                let field = if faces.iter().any(|f| f.weight == weight) {
                    "fontStyle"
                } else {
                    "fontWeight"
                };
                return Err(StorageError::validation(field, &format!("No {family:?} {weight} {style} font; imported faces: {available}. Import the face with import_asset font {{family, weight, style}}")));
            }
            many => match font.filter(|base| many.iter().any(|f| f.id == base.id)) {
                Some(base) => base,
                None => {
                    let ids = many
                        .iter()
                        .map(|f| f.id.as_str())
                        .collect::<Vec<_>>()
                        .join(" and ");
                    return Err(StorageError::validation("fontFamily", &format!("Fonts {ids} are both {family:?} {weight} {style}; author fontId of the one you want")));
                }
            },
        };
        out.insert("fontId".into(), Value::String(selected.id.clone()));
        Ok(())
    }

    fn missing_family(&self, family: &str) -> StorageError {
        let families: std::collections::BTreeSet<_> =
            self.fonts.values().map(|f| f.family.as_str()).collect();
        let available = families
            .iter()
            .take(10)
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        let mut error = StorageError::validation(
            "fontFamily",
            &format!("No imported font family {family:?}; imported families: {available}"),
        );
        if let Some(found) = families.iter().find(|f| f.eq_ignore_ascii_case(family)) {
            error.node_context().suggestion = Some((*found).into());
        }
        error
    }
}
