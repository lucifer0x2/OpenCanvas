//! Validation of an untrusted database and its exact resource set before writes.
use crate::{bundle::*, *};

pub(crate) fn database(path: &Path, manifest: &BundleManifest) -> Result<Snapshot> {
    let mut conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    conn.pragma_update(None, "trusted_schema", "OFF")?;
    conn.pragma_update(None, "query_only", "ON")?;
    // Reject triggers, views, virtual tables and altered schemas before querying
    // authored tables or ever opening this file for writes.
    let mut reference = Connection::open_in_memory()?;
    initialize(&mut reference, "reference", "reference", "Reference")?;
    let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if !(1..=SCHEMA_VERSION).contains(&version) || version != manifest.database_schema_version {
        return Err(invalid("Unsupported database version"));
    }
    if version < 3 {
        reference.execute_batch("DROP TABLE notes")?;
    }
    if version < 2 {
        reference.execute_batch("DROP TABLE retention_metadata")?;
    }
    if schema(&conn)? != schema(&reference)? {
        return Err(invalid("Unexpected database schema objects"));
    }
    let integrity: String = conn.query_row("PRAGMA integrity_check(1)", [], |r| r.get(0))?;
    let foreign_keys: bool = conn.prepare("PRAGMA foreign_key_check")?.exists([])?;
    if integrity != "ok" || foreign_keys {
        return Err(invalid("Database integrity or version check failed"));
    }
    // Only the private staging DB is upgraded, after its exact legacy schema passed.
    if version < SCHEMA_VERSION {
        drop(conn);
        conn = Connection::open(path)?;
        conn.pragma_update(None, "trusted_schema", "OFF")?;
        migration::migrate(&mut conn)?;
        conn.pragma_update(None, "query_only", "ON")?;
    }
    bundle_history::preflight(&conn)?;
    let snapshot = read_snapshot(&mut conn)?;
    if snapshot.project_id != manifest.source_project_id
        || snapshot.revision != manifest.source_revision
        || snapshot.schema_version != SCHEMA_VERSION
        || validate_slug(&snapshot.slug).is_err()
        || snapshot.name.is_empty()
        || snapshot.name.len() > 256
    {
        return Err(invalid("Database identity differs from manifest"));
    }
    let revision: i64 = conn.query_row("SELECT current_revision FROM project_meta", [], |r| {
        r.get(0)
    })?;
    if revision < 0
        || snapshot.assets.len() > MAX_FILES - 3
        || snapshot.tokens.len() > MAX_NODES
        || snapshot.styles.len() > MAX_NODES
        || snapshot.fonts.len() > MAX_FILES
    {
        return Err(invalid("Database exceeds resource or revision bounds"));
    }
    for table in ["tokens", "styles", "assets", "fonts", "notes"] {
        let mut stmt = conn.prepare(&format!("SELECT id,data_json FROM {table}"))?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let value: Value = serde_json::from_str(&row.get::<_, String>(1)?)?;
            if !validate::valid_id(&id)
                || value.get("id").and_then(Value::as_str) != Some(id.as_str())
            {
                return Err(invalid("Resource JSON identity differs from row"));
            }
        }
    }
    let mut stmt = conn.prepare("SELECT id,content_hash,relative_key FROM assets")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (id, digest, key) = row?;
        let asset = snapshot
            .assets
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| invalid("Asset identity mismatch"))?;
        if asset.content_hash != digest || asset.relative_key != key {
            return Err(invalid("Asset columns differ from JSON"));
        }
    }
    Candidate::from_snapshot(&snapshot).validate()?;
    bundle_history::validate(&conn, &snapshot)?;
    Ok(snapshot)
}

type SchemaObject = (String, String, String, Option<String>);

fn schema(conn: &Connection) -> Result<Vec<SchemaObject>> {
    let mut stmt =
        conn.prepare("SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY type,name")?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
    rows.map(|r| Ok(r?)).collect()
}

pub(crate) fn resources(snapshot: &Snapshot, files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    let bundled_hash = hash(BUNDLED_FONT);
    let mut expected = BTreeSet::from([
        "db.sql".to_string(),
        "licenses/OFL.txt".to_string(),
        format!("assets/{bundled_hash}"),
    ]);
    for asset in &snapshot.assets {
        if !valid_hash(&asset.relative_key)
            || asset.relative_key != asset.content_hash
            || asset.status != "ready"
            || asset.byte_size == 0
            || asset.byte_size > MAX_ASSET_BYTES as u64
        {
            return Err(invalid("Invalid immutable asset metadata"));
        }
        let key = format!("assets/{}", asset.relative_key);
        let binary = files
            .get(&key)
            .ok_or_else(|| invalid("Asset binary is missing"))?;
        if binary.len() as u64 != asset.byte_size || hash(binary) != asset.content_hash {
            return Err(invalid("Asset binary differs from snapshot"));
        }
        let dimensions = assets::asset_dimensions(&asset.media_type, binary)?;
        if dimensions != (asset.width, asset.height) {
            return Err(invalid("Asset dimensions differ from binary"));
        }
        expected.insert(key);
    }
    for font in &snapshot.fonts {
        if font.status != "ready"
            || font.family.is_empty()
            || font.family.len() > 128
            || !(100..=900).contains(&font.weight)
            || !["normal", "italic"].contains(&font.style.as_str())
        {
            return Err(invalid("Invalid font metadata"));
        }
        match (&font.asset_id, &font.bundled_id, &font.content_hash) {
            (Some(id), None, Some(digest)) => {
                let asset = snapshot
                    .assets
                    .iter()
                    .find(|a| &a.id == id)
                    .ok_or_else(|| invalid("Font asset is missing"))?;
                if &asset.content_hash != digest || !asset.media_type.starts_with("font/") {
                    return Err(invalid("Font hash differs from asset"));
                }
            }
            (None, Some(id), Some(digest)) if id == "font_bundled" && digest == &bundled_hash => (),
            _ => return Err(invalid("Unknown or mismatched font binary")),
        }
    }
    if files.keys().cloned().collect::<BTreeSet<_>>() != expected
        || files
            .get(&format!("assets/{bundled_hash}"))
            .is_none_or(|b| hash(b) != bundled_hash)
        || files
            .get("licenses/OFL.txt")
            .is_none_or(|b| b.as_slice() != include_bytes!("../../../renderer/fonts/OFL.txt"))
    {
        return Err(invalid("Bundle resources do not exactly match snapshot"));
    }
    Ok(())
}
