//! Validate bounded metadata and durable replay before accepting a database.
use crate::{bundle::*, *};

pub(crate) fn preflight(conn: &Connection) -> Result<()> {
    for (table, maximum) in [
        ("project_meta", 1),
        ("retention_metadata", 1),
        ("nodes", MAX_NODES),
        ("tokens", 4096),
        ("styles", MAX_NODES),
        ("assets", MAX_FILES - 3),
        ("fonts", MAX_FILES),
        ("notes", MAX_NOTES),
    ] {
        let count: i64 =
            conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?;
        if count > maximum as i64 || (maximum == 1 && count != 1) {
            return Err(invalid("Database table exceeds supported row bounds"));
        }
    }
    Ok(())
}

pub(crate) fn validate(conn: &Connection, snapshot: &Snapshot) -> Result<()> {
    // There is no history expiration/GC in this format. Commits form 1..=R;
    // each has exactly one durable idempotency result for the same identity.
    let count: i64 = conn.query_row("SELECT count(*) FROM idempotency", [], |r| r.get(0))?;
    if count as u64 != snapshot.revision {
        return Err(invalid(
            "Replay history does not cover the captured revision",
        ));
    }
    let mut stmt = conn.prepare("SELECT c.revision,c.base_revision,c.batch_id,c.ops_json,c.summary_json,i.request_hash,i.committed_revision,i.response_json FROM commits c LEFT JOIN idempotency i ON i.key=c.batch_id ORDER BY c.revision")?;
    let mut rows = stmt.query([])?;
    let mut expected = 1u64;
    while let Some(row) = rows.next()? {
        let revision: i64 = row.get(0)?;
        let base: i64 = row.get(1)?;
        let key: String = row.get(2)?;
        let history: Value = serde_json::from_str(&row.get::<_, String>(3)?)?;
        let operations = scoped::operations(history.clone());
        let changed: Vec<String> = serde_json::from_str(&row.get::<_, String>(4)?)?;
        let request_hash: String = row.get(5)?;
        let committed: i64 = row.get(6)?;
        let result: BatchResult = serde_json::from_str(&row.get::<_, String>(7)?)?;
        if revision < 1
            || revision as u64 != expected
            || base != revision - 1
            || committed != revision
            || !validate::valid_id(&key)
            || !valid_hash(&request_hash)
            || operations
                .as_array()
                .is_none_or(|ops| ops.is_empty() || ops.len() > MAX_BATCH_OPS)
            || changed.iter().any(|id| !validate::valid_id(id))
            || result.project_id != snapshot.project_id
            || result.committed_revision != expected
            || result.changed_ids != changed
            || result.replayed
            || result.render_status != "pending"
        {
            return Err(invalid("Invalid durable commit or idempotency metadata"));
        }
        if history.is_object() {
            scoped::coverage(&history, &snapshot.project_id, base as u64)?;
        }
        let request = replay_request(&snapshot.project_id, base, &key, history)?;
        if hash(&serde_json::to_vec(&canonical_json(&request))?) != request_hash {
            return Err(invalid(
                "Replay request hash differs from durable operations",
            ));
        }
        expected += 1;
    }
    if expected - 1 != snapshot.revision {
        return Err(invalid(
            "Commit history does not cover the captured revision",
        ));
    }
    // Deletion/reparenting may leave gaps. Keep authored numeric order keys;
    // readers use (sort_key,id) to make sibling order deterministic.
    if snapshot.nodes.iter().any(|node| node.sort_key < 0) {
        return Err(invalid("Invalid sibling ordering metadata"));
    }
    Ok(())
}

pub(crate) fn replay_request(
    project: &str,
    base: i64,
    key: &str,
    operations: Value,
) -> Result<Value> {
    if operations.is_object() {
        let object = operations.as_object().unwrap();
        if operations["scopeVersion"] != 1
            || object.len() != 5
            || ![
                "scopeVersion",
                "request",
                "operations",
                "scopes",
                "scopeHash",
            ]
            .iter()
            .all(|key| object.contains_key(*key))
        {
            return Err(invalid("Unsupported scoped history envelope"));
        }
        let _: Vec<String> = serde_json::from_value(operations["scopes"].clone())?;
        let request = operations["request"].clone();
        let expected = request["expectedRevision"]
            .as_u64()
            .filter(|r| *r <= base as u64)
            .ok_or_else(|| invalid("Invalid scoped history base revision"))?;
        let plain = &operations["operations"];
        if request.get("pageIds").is_some() {
            let scoped: ScopedBatchRequest = serde_json::from_value(request.clone())?;
            if scoped.page_ids.is_empty()
                || scoped.page_ids.len() > 100
                || scoped.page_ids.iter().any(|id| !validate::valid_id(id))
                || scoped.page_ids.iter().collect::<BTreeSet<_>>().len() != scoped.page_ids.len()
                || scoped.project_id != project
                || scoped.idempotency_key != key
                || request["operations"] != *plain
            {
                return Err(invalid("Invalid scoped replay request"));
            }
        } else if expected != base as u64
            || request != replay_request(project, base, key, plain.clone())?
        {
            return Err(invalid("History envelope differs from canonical request"));
        }
        return Ok(request);
    }
    if operations[0]["op"] == "import_asset" {
        if operations.as_array().is_none_or(|ops| ops.len() != 1) {
            return Err(invalid("Invalid resource commit"));
        }
        let asset: Asset = serde_json::from_value(operations[0]["asset"].clone())?;
        let font: Option<FontRegistration> = serde_json::from_value(operations[0]["font"].clone())?;
        return Ok(
            json!({"projectId":project,"expectedRevision":base,"mediaType":asset.media_type,"hash":asset.content_hash,"font":font}),
        );
    }
    let _: Vec<Operation> = serde_json::from_value(operations.clone())?;
    Ok(
        json!({"projectId":project,"expectedRevision":base,"idempotencyKey":key,"operations":operations}),
    )
}
