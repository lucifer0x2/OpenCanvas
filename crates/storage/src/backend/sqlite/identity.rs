use crate::{read_meta, Project, ProjectMeta, Result, Snapshot, SnapshotCacheIdentity};

pub(super) fn snapshot_meta(snapshot: &Snapshot) -> ProjectMeta {
    ProjectMeta {
        project_id: snapshot.project_id.clone(),
        slug: snapshot.slug.clone(),
        name: snapshot.name.clone(),
        schema_version: snapshot.schema_version,
        revision: snapshot.revision,
        // This projection only supplies the authored identity to the pin/cache paths.
        privacy_status: crate::PrivacyStatus::Private,
    }
}

pub(super) fn probe(project: &Project) -> Result<(ProjectMeta, Option<SnapshotCacheIdentity>)> {
    let stamp = file_stamp(project)?;
    let mut reader = project.reader()?;
    let tx = reader.transaction()?;
    let meta = read_meta(&tx)?;
    let asset_keys = {
        let mut stmt = tx.prepare("SELECT relative_key FROM assets ORDER BY id")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()?
    };
    tx.commit()?;
    let readiness = asset_keys
        .iter()
        .map(|key| project.asset_path(key).is_ok())
        .collect();
    let identity = unchanged_identity(project, &meta, stamp, readiness)?;
    Ok((meta, identity))
}

pub(super) fn snapshot(
    project: &Project,
) -> Result<(ProjectMeta, (Snapshot, Option<SnapshotCacheIdentity>))> {
    let stamp = file_stamp(project)?;
    let snapshot = project.snapshot()?;
    let meta = snapshot_meta(&snapshot);
    let readiness = snapshot
        .assets
        .iter()
        .map(|asset| asset.status == "ready")
        .collect();
    let identity = unchanged_identity(project, &meta, stamp, readiness)?;
    Ok((meta, (snapshot, identity)))
}

fn unchanged_identity(
    project: &Project,
    meta: &ProjectMeta,
    stamp: Option<String>,
    asset_readiness: Vec<bool>,
) -> Result<Option<SnapshotCacheIdentity>> {
    let current = file_stamp(project)?;
    Ok(stamp
        .filter(|prior| Some(prior) == current.as_ref())
        .map(|incarnation| SnapshotCacheIdentity {
            project_id: meta.project_id.clone(),
            schema_version: meta.schema_version,
            revision: meta.revision,
            incarnation,
            asset_readiness,
        }))
}

#[cfg(unix)]
fn file_stamp(project: &Project) -> Result<Option<String>> {
    use std::os::unix::fs::MetadataExt;
    // The reader validates containment. Bracket that read with stamps so a file
    // replacement during a read cannot attach bytes to the wrong incarnation.
    let metadata = std::fs::symlink_metadata(project.path.join("db.sql"))?;
    Ok(Some(format!(
        "sqlite:{}:{}:{}:{}",
        metadata.dev(),
        metadata.ino(),
        metadata.ctime(),
        metadata.ctime_nsec()
    )))
}

#[cfg(not(unix))]
fn file_stamp(_project: &Project) -> Result<Option<String>> {
    Ok(None)
}
