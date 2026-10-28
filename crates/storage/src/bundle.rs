//! Version 1, bounded uncompressed TAR interchange.
use crate::*;
use serde::{Deserialize, Serialize};

pub const MAX_BUNDLE_BYTES: usize = 128 * 1024 * 1024;
pub(crate) const MAX_FILES: usize = 4096;
pub(crate) const MAX_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const BUNDLED_FONT: &[u8] = include_bytes!("../../../renderer/fonts/NotoSans.ttf");
const FONT_LICENSE: &[u8] = include_bytes!("../../../renderer/fonts/OFL.txt");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BundleFile {
    pub byte_size: u64,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BundleManifest {
    pub bundle_version: u32,
    pub database_schema_version: u32,
    pub source_project_id: String,
    pub source_revision: u64,
    pub files: BTreeMap<String, BundleFile>,
}
pub struct ProjectBundle {
    pub manifest: BundleManifest,
    pub bytes: Vec<u8>,
}

impl Project {
    /// Capture revision R with SQLite backup, then read its immutable resources.
    /// No transaction or writer lock survives into binary I/O or TAR serialization.
    pub fn export_bundle(&self) -> Result<ProjectBundle> {
        self.check_paths()?;
        let staging = tempfile::tempdir_in(&self.root.path)?;
        let db = staging.path().join("db.sql");
        let snapshot = backup::capture(self, &db)?;
        let mut files = BTreeMap::from([("db.sql".into(), fs::read(db)?)]);
        for asset in &snapshot.assets {
            let path = self.asset_path(&asset.relative_key)?;
            let metadata = fs::metadata(&path)?;
            if metadata.len() > MAX_ASSET_BYTES as u64 {
                return Err(invalid("Asset exceeds bundle limit"));
            }
            let bytes = fs::read(path)?;
            if bytes.len() as u64 != asset.byte_size || hash(&bytes) != asset.content_hash {
                return Err(invalid("Asset hash does not match snapshot"));
            }
            files.insert(format!("assets/{}", asset.relative_key), bytes);
            check_budget(&files)?;
        }
        // Bundle the exact pinned fallback bytes too; uploaded font bytes are
        // already present as asset entries referenced by snapshot.fonts.
        files.insert(
            format!("assets/{}", hash(BUNDLED_FONT)),
            BUNDLED_FONT.to_vec(),
        );
        files.insert("licenses/OFL.txt".into(), FONT_LICENSE.to_vec());
        check_budget(&files)?;
        bundle_validation::resources(&snapshot, &files)?;
        let manifest = BundleManifest {
            bundle_version: 1,
            database_schema_version: SCHEMA_VERSION,
            source_project_id: snapshot.project_id,
            source_revision: snapshot.revision,
            files: files
                .iter()
                .map(|(path, bytes)| {
                    (
                        path.clone(),
                        BundleFile {
                            byte_size: bytes.len() as u64,
                            sha256: hash(bytes),
                        },
                    )
                })
                .collect(),
        };
        let mut tar = tar::Builder::new(Vec::new());
        append(&mut tar, "manifest.json", &serde_json::to_vec(&manifest)?)?;
        for (path, bytes) in files {
            append(&mut tar, &path, &bytes)?;
        }
        let bytes = tar.into_inner()?;
        if bytes.len() > MAX_BUNDLE_BYTES {
            return Err(invalid("Bundle exceeds byte limit"));
        }
        Ok(ProjectBundle { manifest, bytes })
    }
}

fn append(tar: &mut tar::Builder<Vec<u8>>, path: &str, bytes: &[u8]) -> Result<()> {
    let mut header = tar::Header::new_ustar();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o600);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    tar.append_data(&mut header, path, bytes)?;
    Ok(())
}

pub(crate) fn invalid(message: &str) -> StorageError {
    StorageError::new("BUNDLE_INVALID", message)
}
pub(crate) fn check_budget(files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    if files.len() > MAX_FILES
        || files.values().map(Vec::len).sum::<usize>() > MAX_BUNDLE_BYTES - MAX_MANIFEST_BYTES
    {
        return Err(invalid("Bundle exceeds file or byte limits"));
    }
    Ok(())
}

pub(crate) fn valid_path(path: &str) -> bool {
    path == "db.sql"
        || path == "licenses/OFL.txt"
        || path.strip_prefix("assets/").is_some_and(valid_hash)
}
pub(crate) fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn decode(bytes: &[u8]) -> Result<(BundleManifest, BTreeMap<String, Vec<u8>>)> {
    decode_archive(bytes).map_err(|_| invalid("Invalid or unsupported project archive"))
}

fn decode_archive(bytes: &[u8]) -> Result<(BundleManifest, BTreeMap<String, Vec<u8>>)> {
    use std::io::Read;
    if bytes.len() > MAX_BUNDLE_BYTES || !bytes.len().is_multiple_of(512) {
        return Err(invalid("Bundle exceeds byte limit"));
    }
    let mut archive = tar::Archive::new(std::io::Cursor::new(bytes));
    let mut files = BTreeMap::new();
    // Raw entries exposes extensions instead of interpreting GNU/PAX path/link
    // rewrites. The public format accepts only plain USTAR regular entries.
    for entry in archive.entries()?.raw(true) {
        let mut entry = entry?;
        let header = entry.header();
        if !header.entry_type().is_file()
            || header.as_ustar().is_none()
            || header.link_name_bytes().is_some()
        {
            return Err(invalid("Only regular USTAR files are allowed"));
        }
        let path = std::str::from_utf8(&header.path_bytes())
            .map_err(|_| invalid("Invalid archive path"))?
            .to_owned();
        if path != "manifest.json" && !valid_path(&path) {
            return Err(invalid("Archive path is not allowed"));
        }
        if files.contains_key(&path) || files.len() > MAX_FILES {
            return Err(invalid("Duplicate or excessive archive entries"));
        }
        let limit = if path == "db.sql" {
            backup::MAX_DB_BYTES
        } else if path == "manifest.json" {
            MAX_MANIFEST_BYTES as u64
        } else {
            MAX_ASSET_BYTES as u64
        };
        if entry.size() > limit {
            return Err(invalid("Archive entry exceeds size limit"));
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        files.insert(path, data);
    }
    let consumed = archive.into_inner().position() as usize;
    if bytes.len().saturating_sub(consumed) < 512 || bytes[consumed..].iter().any(|b| *b != 0) {
        return Err(invalid(
            "Archive must end with zero blocks and no trailing data",
        ));
    }
    let manifest: BundleManifest = serde_json::from_slice(
        &files
            .remove("manifest.json")
            .ok_or_else(|| invalid("Missing bundle manifest"))?,
    )?;
    check_budget(&files)?;
    if manifest.bundle_version != 1
        || manifest.database_schema_version != SCHEMA_VERSION
        || !validate::valid_id(&manifest.source_project_id)
        || manifest.source_revision > i64::MAX as u64
    {
        return Err(invalid("Unsupported bundle schema or identity"));
    }
    if manifest.files.len() != files.len() {
        return Err(invalid("Manifest file set differs from archive"));
    }
    for (path, expected) in &manifest.files {
        let actual = files
            .get(path)
            .ok_or_else(|| invalid("Manifest binary is missing"))?;
        if !valid_path(path)
            || !valid_hash(&expected.sha256)
            || expected.byte_size != actual.len() as u64
            || expected.sha256 != hash(actual)
        {
            return Err(invalid("Bundle file hash or size mismatch"));
        }
    }
    Ok((manifest, files))
}
