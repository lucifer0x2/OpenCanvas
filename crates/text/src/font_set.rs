use crate::{Face, Font, MeasureRequest, Measurement, TextError};
use std::collections::BTreeMap;

/// Exact manifest ID -> original-byte hash + descriptor mapping. No fallback or I/O.
#[derive(Clone, Default)]
pub struct FontSet {
    fonts: BTreeMap<String, Font>,
}

impl FontSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Requires `with_face` first. Rebinding an existing ID to different content/descriptors
    /// is an error; callers construct a fresh set for a changed immutable manifest.
    pub fn insert(&mut self, manifest_id: impl Into<String>, font: Font) -> Result<(), TextError> {
        let id = manifest_id.into();
        if id.is_empty()
            || id.len() > 1024
            || id.chars().any(char::is_control)
            || font.face().is_none()
        {
            return Err(TextError::InvalidFace);
        }
        if let Some(previous) = self.fonts.get(&id) {
            if previous.hash() != font.hash() || previous.face() != font.face() {
                return Err(TextError::InvalidFace);
            }
        }
        self.fonts.insert(id, font);
        Ok(())
    }

    pub fn get(&self, manifest_id: &str) -> Option<&Font> {
        self.fonts.get(manifest_id)
    }

    pub fn measure(&self, request: &MeasureRequest) -> Result<Measurement, TextError> {
        let font = self.get(&request.font_id).ok_or(TextError::FontMissing)?;
        let expected = Face {
            family: request.family.clone(),
            weight: request.weight,
            style: request.style.clone(),
        };
        if font.hash() != request.font_hash || font.face() != Some(&expected) {
            return Err(TextError::FontMissing);
        }
        crate::measure::measure(font, request)
    }
}
