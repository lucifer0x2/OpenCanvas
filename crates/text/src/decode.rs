//! Bounded WOFF2 reconstruction and structural SFNT validation, not an OTS replacement.
use crate::TextError;
use read_fonts::{FontRef, TableProvider};
use std::collections::BTreeSet;

pub const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_SFNT_BYTES: usize = 128 * 1024 * 1024;

pub fn decode(bytes: &[u8]) -> Result<Vec<u8>, TextError> {
    if bytes.len() > MAX_FONT_BYTES {
        return Err(TextError::FontLimit);
    }
    let sfnt = if bytes.starts_with(b"wOF2") {
        // Wuff 0.2.9 bounds directory-derived decompression/reconstruction to 128 MiB,
        // with a fixed-size Brotli output and <=100x compression ratio BEFORE allocation.
        // Reject collections: the product manifest describes one exact face.
        if bytes.len() < 48 || &bytes[4..8] == b"ttcf" {
            return Err(TextError::FontDecode);
        }
        wuff::decompress_woff2(bytes).map_err(|_| TextError::FontDecode)?
    } else {
        bytes.to_vec()
    };
    validate(&sfnt)?;
    Ok(sfnt)
}

fn validate(bytes: &[u8]) -> Result<(), TextError> {
    let invalid = || TextError::FontDecode;
    if bytes.len() > MAX_SFNT_BYTES {
        return Err(TextError::FontLimit);
    }
    if bytes.len() < 12 || !matches!(&bytes[..4], b"\0\x01\0\0" | b"OTTO" | b"true") {
        return Err(invalid());
    }
    let count = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    let end = 12 + count * 16;
    if count == 0 || count > 4095 || end > bytes.len() {
        return Err(invalid());
    }
    let mut tags = BTreeSet::new();
    let mut ranges = Vec::new();
    for record in bytes[12..end].as_chunks::<16>().0 {
        let tag: [u8; 4] = record[..4].try_into().map_err(|_| invalid())?;
        let at = u32::from_be_bytes(record[8..12].try_into().map_err(|_| invalid())?) as usize;
        let len = u32::from_be_bytes(record[12..16].try_into().map_err(|_| invalid())?) as usize;
        if !tags.insert(tag) || at < end || at > bytes.len() || len > bytes.len() - at {
            return Err(invalid());
        }
        if len > 0 {
            ranges.push((at, at + len));
        }
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|r| r[0].1 > r[1].0) {
        return Err(invalid());
    }
    let font = FontRef::new(bytes).map_err(|_| invalid())?;
    let head = font.head().map_err(|_| invalid())?;
    let maxp = font.maxp().map_err(|_| invalid())?;
    let hhea = font.hhea().map_err(|_| invalid())?;
    font.hmtx().map_err(|_| invalid())?;
    font.cmap().map_err(|_| invalid())?;
    if !(16..=16384).contains(&head.units_per_em())
        || maxp.num_glyphs() == 0
        || hhea.number_of_h_metrics() == 0
        || hhea.number_of_h_metrics() > maxp.num_glyphs()
        || !(tags.contains(b"glyf") && tags.contains(b"loca")
            || tags.contains(b"CFF ")
            || tags.contains(b"CFF2"))
    {
        return Err(invalid());
    }
    Ok(())
}
