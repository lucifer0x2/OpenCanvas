use super::*;
use crate::{Retention, Snapshot, SnapshotCache};
use ::postgres::GenericClient;
use std::{collections::BTreeSet, sync::Arc};

mod stored;
pub(super) use stored::Stored;

const ENTITIES: &str = "SELECT kind,id,data FROM entities WHERE project_id=$1 UNION ALL SELECT 'binary',content_hash,'' FROM binaries WHERE project_id=$1";

/// A snapshot with the generation and namespace that authorized it.
pub(super) type Identified = (Arc<Snapshot>, i64, String);

/// Content hashes with a stored binary for one project.
pub(super) type Binaries = BTreeSet<String>;

pub(super) enum Read {
    /// Served from the cache after this call's successful SQL probe.
    Hit(Identified),
    /// Loaded from SQL; the identity comes from the same transaction.
    Loaded(Identified),
}

/// With a cache, an autocommit identity probe precedes any hit. A miss then
/// loads the canonical ID in one repeatable-read transaction on this worker.
pub(super) fn read_through(
    session: &mut Session,
    key: &str,
    cache: Option<&SnapshotCache>,
    namespace: &str,
) -> Result<Read> {
    let Some(cache) = cache else {
        return read(session, key).map(Read::Loaded);
    };
    let current = projects::lookup(&mut session.db(), key, false)?;
    if current.namespace == namespace {
        let (id, generation) = (&current.meta.project_id, current.generation);
        if let Some(snapshot) = cache.get(id, generation, current.meta.revision) {
            return Ok(Read::Hit((snapshot, generation, current.namespace)));
        }
    }
    read(session, &current.meta.project_id).map(Read::Loaded)
}

fn read(session: &mut Session, key: &str) -> Result<Identified> {
    let (mut tx, statements) = session.read_only()?;
    let mut db = Db::new(&mut tx, statements);
    let project = projects::lookup(&mut db, key, false)?;
    let generation = project.generation;
    let namespace = project.namespace.clone();
    let snapshot = entities(&mut db, project, true)?;
    tx.commit().map_err(sql)?;
    Ok((Arc::new(snapshot), generation, namespace))
}

pub(super) fn entities<C: GenericClient>(
    db: &mut Db<'_, C>,
    project: projects::ProjectRow,
    readiness: bool,
) -> Result<Snapshot> {
    let (mut snapshot, binaries) = entities_with_binaries(db, project)?;
    if readiness {
        apply_readiness(&mut snapshot, &binaries);
    }
    Ok(snapshot)
}

/// Entities as stored, plus the project's binary hashes from the same query.
pub(super) fn entities_with_binaries<C: GenericClient>(
    db: &mut Db<'_, C>,
    project: projects::ProjectRow,
) -> Result<(Snapshot, Binaries)> {
    load(db, project, None)
}

/// As `entities_with_binaries`, also keeping each row's stored text for a write.
pub(super) fn entities_for_write<C: GenericClient>(
    db: &mut Db<'_, C>,
    project: projects::ProjectRow,
) -> Result<(Snapshot, Binaries, Stored)> {
    let mut stored = Stored::default();
    let (snapshot, binaries) = load(db, project, Some(&mut stored))?;
    stored.sort();
    Ok((snapshot, binaries, stored))
}

fn load<C: GenericClient>(
    db: &mut Db<'_, C>,
    project: projects::ProjectRow,
    mut stored: Option<&mut Stored>,
) -> Result<(Snapshot, Binaries)> {
    let mut snapshot = header(project);
    let mut binaries = Binaries::new();
    for row in db.query(ENTITIES, &[&snapshot.project_id])? {
        let kind: &str = row.get(0);
        let data: &str = row.get(2);
        macro_rules! push {
            ($field:ident) => {{
                snapshot.$field.push(serde_json::from_str(data)?);
                if let (Some(stored), Some(entity)) = (stored.as_mut(), snapshot.$field.last()) {
                    stored.$field.push((entity.id.clone(), data.to_owned()));
                }
            }};
        }
        match kind {
            "node" => push!(nodes),
            "token" => push!(tokens),
            "style" => push!(styles),
            "asset" => push!(assets),
            "font" => push!(fonts),
            "note" => push!(notes),
            "binary" => {
                binaries.insert(row.get(1));
            }
            _ => {
                return Err(crate::StorageError::new(
                    "STORAGE_FAILED",
                    "Project contains unsupported entity data",
                ))
            }
        }
    }
    sort(&mut snapshot);
    Ok((snapshot, binaries))
}

fn header(project: projects::ProjectRow) -> Snapshot {
    let meta = project.meta;
    Snapshot {
        schema_version: meta.schema_version,
        project_id: meta.project_id,
        slug: meta.slug,
        name: meta.name,
        revision: meta.revision,
        nodes: vec![],
        tokens: vec![],
        styles: vec![],
        assets: vec![],
        fonts: vec![],
        notes: vec![],
        retention: Retention {
            oldest_commit_revision: project.oldest,
            idempotency_policy: "Retained for project lifetime; no expiration".into(),
        },
    }
}

/// The public entity order, independent of input order.
pub(super) fn sort(snapshot: &mut Snapshot) {
    snapshot
        .nodes
        .sort_by(|a, b| (&a.parent_id, a.sort_key, &a.id).cmp(&(&b.parent_id, b.sort_key, &b.id)));
    // SQL text ordering follows the database locale; public arrays use Rust's
    // bytewise order regardless of how PostgreSQL was initialized.
    snapshot.tokens.sort_by(|a, b| a.id.cmp(&b.id));
    snapshot.styles.sort_by(|a, b| a.id.cmp(&b.id));
    snapshot.assets.sort_by(|a, b| a.id.cmp(&b.id));
    snapshot.fonts.sort_by(|a, b| a.id.cmp(&b.id));
    snapshot.notes.sort_by(|a, b| a.id.cmp(&b.id));
}

/// Asset status follows its binary; an asset-backed font follows its asset.
/// Bundled fonts keep their stored status. Requires `sort` first.
pub(super) fn apply_readiness(snapshot: &mut Snapshot, binaries: &Binaries) {
    for asset in &mut snapshot.assets {
        asset.status = if binaries.contains(&asset.content_hash) {
            "ready"
        } else {
            "missing"
        }
        .into();
    }
    for font in &mut snapshot.fonts {
        if let Some(id) = &font.asset_id {
            // Assets are sorted by id; the lower bound is the first match.
            let assets = &snapshot.assets;
            let index = assets.partition_point(|asset| &asset.id < id);
            font.status = assets
                .get(index)
                .filter(|asset| &asset.id == id)
                .map_or("missing", |asset| asset.status.as_str())
                .into();
        }
    }
}
