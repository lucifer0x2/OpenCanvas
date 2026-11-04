use crate::{Font, TextError, MAX_RUN_BYTES};
use std::collections::BTreeSet;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

// Unicode Default_Ignorable_Code_Point (not all Cf; not just variation selectors).
pub(crate) fn default_ignorable(c: char) -> bool {
    matches!(c as u32,
        0x00ad | 0x034f | 0x061c | 0x115f..=0x1160 | 0x17b4..=0x17b5
        | 0x180b..=0x180f | 0x200b..=0x200f | 0x202a..=0x202e | 0x2060..=0x206f
        | 0x3164 | 0xfe00..=0xfe0f | 0xfeff | 0xffa0 | 0xfff0..=0xfff8
        | 0x1bca0..=0x1bca3 | 0x1d173..=0x1d17a | 0xe0000..=0xe0fff)
}

fn ignored(c: char) -> bool {
    c.is_whitespace() || c.is_control() || default_ignorable(c)
}

impl Font {
    /// Sorted unique missing scalar values. Mirrors renderer NFC-per-grapheme coverage;
    /// this does not claim emoji sequence/variation support. Bounded by text byte limit.
    pub fn missing_codepoints(&self, text: &str) -> Result<Vec<u32>, TextError> {
        if text.len() > MAX_RUN_BYTES {
            return Err(TextError::TextLimit);
        }
        let charmap = self.inner.charmap();
        let covered = |c: char| {
            charmap
                .map_unicode(c)
                .is_some_and(|glyph| glyph.to_u32() != 0)
        };
        let mut missing = BTreeSet::new();
        for segment in text.graphemes(true) {
            if segment.chars().filter(|c| !ignored(*c)).all(covered) {
                continue;
            }
            let canonical: Vec<char> = segment.nfc().filter(|c| !ignored(*c)).collect();
            if canonical.iter().copied().all(covered) {
                continue;
            }
            missing.extend(
                canonical
                    .into_iter()
                    .filter(|c| !covered(*c))
                    .map(u32::from),
            );
        }
        Ok(missing.into_iter().collect())
    }
}
