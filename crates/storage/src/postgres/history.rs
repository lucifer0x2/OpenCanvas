use super::*;
use crate::{Commit, StorageError};

const AFTER: &str = "SELECT revision,base_revision,batch_id,operations,changed_ids FROM commits WHERE project_id=$1 AND revision>$2 ORDER BY revision LIMIT $3";

pub(super) fn after(
    session: &mut Session,
    key: &str,
    revision: u64,
    limit: usize,
) -> Result<Vec<Commit>> {
    let (mut tx, statements) = session.read_only()?;
    let mut db = Db::new(&mut tx, statements);
    let project = projects::lookup(&mut db, key, false)?;
    if revision > project.meta.revision || revision.saturating_add(1) < project.oldest {
        let mut error = StorageError::new(
            "RESET_REQUIRED",
            "Event cursor is unavailable; refetch snapshot",
        );
        error.current_revision = Some(project.meta.revision);
        return Err(error);
    }
    let rows = db.query(
        AFTER,
        &[
            &project.meta.project_id,
            &(revision as i64),
            &(limit.clamp(1, 1000) as i64),
        ],
    )?;
    let commits = rows
        .into_iter()
        .map(|row| {
            Ok(Commit {
                revision: row.get::<_, i64>(0) as u64,
                base_revision: row.get::<_, i64>(1) as u64,
                batch_id: row.get(2),
                operations: crate::scoped::operations(serde_json::from_str(
                    &row.get::<_, String>(3),
                )?),
                changed_ids: serde_json::from_str(&row.get::<_, String>(4))?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    tx.commit().map_err(sql)?;
    Ok(commits)
}
