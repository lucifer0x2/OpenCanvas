use super::*;
use crate::{
    validate::Candidate, Asset, BatchResult, Font, ImportAssetRequest, ImportAssetResult,
    StorageError, MAX_ASSET_BYTES,
};
use ::postgres::GenericClient;
use serde_json::json;
use write_through::Committed;

const ENTITY: &str = "SELECT data FROM entities WHERE project_id=$1 AND kind=$2 AND id=$3";
const BINARY: &str = "SELECT CASE WHEN octet_length(bytes)<=$3 THEN bytes END FROM binaries WHERE project_id=$1 AND content_hash=$2";
const INSERT_BINARY: &str = "INSERT INTO binaries(project_id,content_hash,bytes) VALUES($1,$2,$3)";

pub(super) fn import(
    session: &mut Session,
    key: &str,
    request: ImportAssetRequest,
) -> Result<(ImportAssetResult, Option<Box<Committed>>)> {
    let mut tx = session.client.transaction().map_err(sql)?;
    let mut db = Db::new(&mut tx, &mut session.statements);
    let project = projects::lookup(&mut db, key, true)?;
    transaction::request_ids(
        &project.meta.project_id,
        &request.project_id,
        &request.idempotency_key,
    )?;
    if request.bytes.is_empty() || request.bytes.len() > MAX_ASSET_BYTES {
        return Err(StorageError::validation(
            "bytes",
            "Asset must contain 1 byte to 10 MiB",
        ));
    }
    let (width, height) = crate::assets::asset_dimensions(&request.media_type, &request.bytes)?;
    let content_hash = crate::hash(&request.bytes);
    let value = crate::canonical_json(&json!({"projectId": request.project_id,
        "expectedRevision": request.expected_revision,"mediaType": request.media_type,
        "hash": content_hash,"font": request.font}));
    let hash = crate::hash(&serde_json::to_vec(&value)?);
    if let Some((mut result, asset)) = transaction::replay(
        &mut db,
        &project.meta.project_id,
        &request.idempotency_key,
        &hash,
    )? {
        result.replayed = true;
        // The original manifest is part of replay, never reconstructed from current rows.
        let asset = asset.ok_or_else(|| {
            StorageError::new("STORAGE_FAILED", "Stored import response is unavailable")
        })?;
        return Ok((ImportAssetResult { asset, result }, None));
    }
    if project.meta.revision != request.expected_revision {
        return Err(StorageError::conflict(project.meta.revision));
    }
    let identity = (project.generation, project.namespace.clone());
    let (before, mut binaries, stored) = snapshot::entities_for_write(&mut db, project)?;
    let mut candidate = Candidate::from_snapshot(&before);
    let asset = Asset {
        id: format!("asset_{}", &content_hash[..32]),
        content_hash: content_hash.clone(),
        media_type: request.media_type.clone(),
        byte_size: request.bytes.len() as u64,
        relative_key: content_hash.clone(),
        width,
        height,
        status: "ready".into(),
    };
    candidate.assets.insert(asset.id.clone(), asset.clone());
    let mut changed_ids = vec![asset.id.clone()];
    crate::assets::register_font(&mut candidate, &request, &asset, &mut changed_ids)?;
    candidate.validate()?;
    let existing = db.query_opt(
        BINARY,
        &[&before.project_id, &content_hash, &(MAX_ASSET_BYTES as i32)],
    )?;
    if let Some(row) = existing {
        let bytes: Vec<u8> = row.get::<_, Option<Vec<u8>>>(0).ok_or_else(|| {
            StorageError::new("ASSET_INVALID", "Asset exceeds the supported size")
        })?;
        if crate::hash(&bytes) != content_hash {
            return Err(StorageError::new(
                "ASSET_INVALID",
                "Existing asset content has a different hash",
            ));
        }
    } else {
        db.execute(
            INSERT_BINARY,
            &[&before.project_id, &content_hash, &request.bytes],
        )?;
    }
    binaries.insert(content_hash.clone());
    let result = BatchResult {
        project_id: before.project_id.clone(),
        committed_revision: before.revision + 1,
        changed_ids,
        render_status: "pending".into(),
        replayed: false,
    };
    let rows = stored.delta(&candidate)?;
    drop((stored, candidate));
    let operations = json!([{"op":"import_asset","asset":asset,"font":request.font}]);
    let written = transaction::persist(
        &mut db,
        &before,
        rows,
        &request.idempotency_key,
        &hash,
        &operations,
        &result,
        Some(&asset),
    )?;
    tx.commit().map_err(sql)?;
    let revision = result.committed_revision;
    let committed = Committed::new(before, binaries, written, revision, identity);
    Ok((
        ImportAssetResult { asset, result },
        Some(Box::new(committed)),
    ))
}

fn entity<T: serde::de::DeserializeOwned, C: GenericClient>(
    db: &mut Db<'_, C>,
    project: &str,
    kind: &str,
    id: &str,
    code: &str,
    message: &str,
) -> Result<T> {
    // Arbitrary resource IDs are valid lookup misses. NUL cannot be SQL TEXT.
    if id.contains('\0') {
        return Err(StorageError::new(code, message));
    }
    let row = db
        .query_opt(ENTITY, &[&project, &kind, &id])?
        .ok_or_else(|| StorageError::new(code, message))?;
    Ok(serde_json::from_str(&row.get::<_, String>(0))?)
}

fn asset<C: GenericClient>(
    db: &mut Db<'_, C>,
    project: &str,
    id: &str,
) -> Result<(Asset, Vec<u8>)> {
    let asset: Asset = entity(
        db,
        project,
        "asset",
        id,
        "ASSET_NOT_FOUND",
        "Asset does not exist",
    )?;
    let key = &asset.relative_key;
    if key.len() != 64 || !key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(StorageError::new(
            "ASSET_MISSING",
            "Asset binary is unavailable",
        ));
    }
    let row = db
        .query_opt(BINARY, &[&project, key, &(MAX_ASSET_BYTES as i32)])?
        .ok_or_else(|| StorageError::new("ASSET_MISSING", "Asset binary is unavailable"))?;
    let bytes: Vec<u8> = row
        .get::<_, Option<Vec<u8>>>(0)
        .ok_or_else(|| StorageError::new("ASSET_INVALID", "Asset exceeds the supported size"))?;
    if bytes.len() > MAX_ASSET_BYTES
        || bytes.len() as u64 != asset.byte_size
        || crate::hash(&bytes) != asset.content_hash
    {
        return Err(StorageError::new(
            "ASSET_INVALID",
            "Asset content does not match its manifest hash",
        ));
    }
    Ok((asset, bytes))
}

pub(super) fn read_asset(session: &mut Session, key: &str, id: &str) -> Result<(Asset, Vec<u8>)> {
    let (mut tx, statements) = session.read_only()?;
    let mut db = Db::new(&mut tx, statements);
    let project = projects::lookup(&mut db, key, false)?;
    let result = asset(&mut db, &project.meta.project_id, id)?;
    tx.commit().map_err(sql)?;
    Ok(result)
}

pub(super) fn read_font(
    session: &mut Session,
    key: &str,
    id: &str,
) -> Result<(Font, String, Vec<u8>)> {
    let (mut tx, statements) = session.read_only()?;
    let mut db = Db::new(&mut tx, statements);
    let project = projects::lookup(&mut db, key, false)?;
    let font: Font = entity(
        &mut db,
        &project.meta.project_id,
        "font",
        id,
        "FONT_NOT_FOUND",
        "Font does not exist",
    )?;
    let (mime, bytes) = match (&font.asset_id, &font.bundled_id) {
        (Some(id), None) => {
            let (asset, bytes) = asset(&mut db, &project.meta.project_id, id)?;
            if !asset.media_type.starts_with("font/") {
                return Err(StorageError::new(
                    "FONT_INVALID",
                    "Font asset is not a font",
                ));
            }
            (asset.media_type, bytes)
        }
        (None, Some(id)) if id == "font_bundled" => {
            ("font/ttf".into(), crate::bundle::BUNDLED_FONT.to_vec())
        }
        _ => {
            return Err(StorageError::new(
                "FONT_MISSING",
                "Font binary is unavailable",
            ))
        }
    };
    if font.status != "ready" || font.content_hash.as_deref() != Some(crate::hash(&bytes).as_str())
    {
        return Err(StorageError::new(
            "FONT_INVALID",
            "Font bytes do not match its manifest",
        ));
    }
    tx.commit().map_err(sql)?;
    Ok((font, mime, bytes))
}
