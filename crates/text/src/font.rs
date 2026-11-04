use crate::{decode, TextError};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Explicit manifest labels, as with CSS FontFace descriptors. No system family lookup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Face {
    pub family: String,
    pub weight: u16,
    pub style: String,
}

/// Verified original identity and structurally checked SFNT, shared cheaply by clones.
#[derive(Clone)]
pub struct Font {
    pub(crate) inner: harfrust::Font,
    sfnt: Arc<Vec<u8>>,
    hash: String,
    face: Option<Face>,
    // User-space values for every axis, retained for avar2 inter-axis mapping.
    location: Vec<(harfrust::Tag, f32)>,
}

impl Font {
    /// Hash ORIGINAL upload bytes before WOFF2 decoding. Accepts only lower-case SHA256.
    pub fn from_bytes(bytes: impl AsRef<[u8]>, expected_hash: &str) -> Result<Self, TextError> {
        if expected_hash.len() != 64
            || !expected_hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(TextError::InvalidHash);
        }
        let bytes = bytes.as_ref();
        if bytes.len() > decode::MAX_FONT_BYTES {
            return Err(TextError::FontLimit);
        }
        let hash = format!("{:x}", Sha256::digest(bytes));
        if hash != expected_hash {
            return Err(TextError::HashMismatch);
        }
        let sfnt = Arc::new(decode::decode(bytes)?);
        let source: Arc<dyn AsRef<[u8]> + Send + Sync> = sfnt.clone();
        let inner = harfrust::Font::new(source, 0).ok_or(TextError::FontDecode)?;
        if !inner.charmap().has_unicode() {
            return Err(TextError::FontDecode);
        }
        let location = inner
            .axes()
            .iter()
            .map(|a| (a.tag(), a.default_value()))
            .collect();
        Ok(Self {
            inner,
            sfnt,
            hash,
            face: None,
            location,
        })
    }

    /// Bind the exact manifest descriptors. Like FontFace, labels need not equal font name
    /// table strings; they must match every measurement request exactly. No synthetic face.
    pub fn with_face(
        mut self,
        family: impl Into<String>,
        weight: u16,
        style: impl Into<String>,
    ) -> Result<Self, TextError> {
        let family = family.into();
        let style = style.into();
        if family.is_empty()
            || family.len() > 1024
            || family.chars().any(char::is_control)
            || !(100..=900).contains(&weight)
            || !matches!(style.as_str(), "normal" | "italic")
        {
            return Err(TextError::InvalidFace);
        }
        for (tag, value) in &mut self.location {
            if *tag == harfrust::Tag::new(b"wght") {
                *value = weight as f32;
            }
        }
        self.inner = crate::optical::at_location(&self.inner, &self.location);
        self.face = Some(Face {
            family,
            weight,
            style,
        });
        Ok(self)
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }
    pub fn face(&self) -> Option<&Face> {
        self.face.as_ref()
    }
    /// Decompressed, directory-validated single-face SFNT bytes, not the original WOFF2.
    pub fn sfnt_bytes(&self) -> &[u8] {
        &self.sfnt
    }
    /// Shared exact font instance for a reusable `ScalerFont::unhinted(font.clone(), None)`.
    pub fn shaping_font(&self) -> &harfrust::Font {
        &self.inner
    }

    /// Canvas uses automatic optical sizing in CSS px. Use this exact instance for
    /// outlines too: `shaping_font()` has manifest weight but no automatic optical size.
    pub fn shaping_font_at_size(&self, size: f32) -> Result<harfrust::Font, TextError> {
        if !size.is_finite() || !(1.0..=10000.0).contains(&size) {
            return Err(TextError::InvalidRequest);
        }
        Ok(crate::optical::at_size(&self.inner, size, &self.location))
    }
}
