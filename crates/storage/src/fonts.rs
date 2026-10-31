//! Font binaries are content-addressed project assets or the pinned bundled face.
use crate::*;

impl Project {
    /// Reads exact original bytes, checking the font and asset manifests together.
    pub fn read_font(&self, id: &str) -> Result<(Font, String, Vec<u8>)> {
        let snapshot = self.snapshot()?;
        let font = snapshot
            .fonts
            .into_iter()
            .find(|f| f.id == id)
            .ok_or_else(|| StorageError::new("FONT_NOT_FOUND", "Font does not exist"))?;
        let (mime, bytes) = match (&font.asset_id, &font.bundled_id) {
            (Some(asset), None) => {
                let (asset, bytes) = self.read_asset(asset)?;
                if !asset.media_type.starts_with("font/") {
                    return Err(StorageError::new(
                        "FONT_INVALID",
                        "Font asset is not a font",
                    ));
                }
                (asset.media_type, bytes)
            }
            (None, Some(id)) if id == "font_bundled" => {
                ("font/ttf".into(), bundle::BUNDLED_FONT.to_vec())
            }
            _ => {
                return Err(StorageError::new(
                    "FONT_MISSING",
                    "Font binary is unavailable",
                ))
            }
        };
        if font.status != "ready" || font.content_hash.as_deref() != Some(hash(&bytes).as_str()) {
            return Err(StorageError::new(
                "FONT_INVALID",
                "Font bytes do not match its manifest",
            ));
        }
        Ok((font, mime, bytes))
    }
}
