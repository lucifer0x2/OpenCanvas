use std::{error::Error, fmt};

/// Pure library failures. Missing glyphs are diagnostics, not errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextError {
    HashMismatch,
    InvalidHash,
    FontDecode,
    FontLimit,
    InvalidFace,
    FontMissing,
    InvalidRequest,
    TextLimit,
    Shape,
}

impl TextError {
    /// Stable pipeline mapping, suitable for server adapters (never includes bytes or paths).
    pub const fn code(&self) -> &'static str {
        match self {
            Self::HashMismatch => "RESOURCE_HASH_MISMATCH",
            Self::InvalidHash | Self::InvalidFace | Self::InvalidRequest => "VALIDATION_FAILED",
            Self::FontDecode | Self::FontMissing => "FONT_MISSING",
            Self::FontLimit | Self::TextLimit => "RESOURCE_LIMIT",
            Self::Shape => "TEXT_SHAPING_FAILED",
        }
    }
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {self:?}", self.code())
    }
}
impl Error for TextError {}
