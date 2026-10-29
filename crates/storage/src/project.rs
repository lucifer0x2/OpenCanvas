use crate::*;
pub struct Project {
    pub(crate) root: Arc<Root>,
    pub(crate) path: PathBuf,
    pub(crate) project_id: String,
    pub(crate) writer: Mutex<Connection>,
}

impl Project {
    pub fn id(&self) -> &str {
        &self.project_id
    }
    pub(crate) fn check_paths(&self) -> Result<()> {
        guard(&self.path, &self.root.path)?;
        guard(&self.path.join("assets"), &self.path)?;
        guard(&self.path.join("db.sql"), &self.path)?;
        no_symlink(&self.path.join("db.sql-wal"))?;
        no_symlink(&self.path.join("db.sql-shm"))?;
        Ok(())
    }
    pub(crate) fn reader(&self) -> Result<Connection> {
        self.check_paths()?;
        let c = Connection::open_with_flags(
            self.path.join("db.sql"),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        configure(&c, false)?;
        Ok(c)
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let mut conn = self.reader()?;
        let mut s = read_snapshot(&mut conn)?;
        // Files can disappear independently of SQLite. Surface readiness separately.
        for asset in &mut s.assets {
            asset.status = if self.asset_path(&asset.relative_key).is_ok() {
                "ready"
            } else {
                "missing"
            }
            .into();
        }
        for font in &mut s.fonts {
            if let Some(asset) = &font.asset_id {
                font.status = s
                    .assets
                    .iter()
                    .find(|a| &a.id == asset)
                    .map_or("missing", |a| a.status.as_str())
                    .into();
            }
        }
        Ok(s)
    }
    pub fn apply_batch(&self, request: BatchRequest) -> Result<BatchResult> {
        self.apply_batch_measured(request).map(|(r, _)| r)
    }
    pub fn apply_batch_measured(
        &self,
        request: BatchRequest,
    ) -> Result<(BatchResult, BatchMetrics)> {
        let total = Instant::now();
        self.check_paths()?;
        self.request_ids(&request.project_id, &request.idempotency_key)?;
        if request.operations.is_empty() || request.operations.len() > MAX_BATCH_OPS {
            return Err(StorageError::validation(
                "operations",
                "Batch requires 1–500 operations",
            ));
        }
        let request_value = serde_json::to_value(&request)?;
        let bytes = serde_json::to_vec(&canonical_json(&request_value))?;
        if bytes.len() > MAX_BATCH_BYTES {
            return Err(StorageError::validation(
                "operations",
                "Batch exceeds 2 MiB",
            ));
        }
        let request_hash = hash(&bytes);
        let wait = Instant::now();
        let mut conn = self.writer.lock().map_err(|_| poisoned())?;
        let mut metrics = BatchMetrics {
            queue_wait_us: us(wait),
            request_bytes: bytes.len(),
            ..Default::default()
        };
        if let Some(mut result) = replay(&conn, &request.idempotency_key, &request_hash)? {
            result.replayed = true;
            metrics.total_us = us(total);
            return Ok((result, metrics));
        }
        let time = Instant::now();
        let before = read_snapshot(&mut conn)?;
        metrics.read_us = us(time);
        if before.revision != request.expected_revision {
            return Err(StorageError::conflict(before.revision));
        }
        let time = Instant::now();
        let mut candidate = Candidate::from_snapshot(&before);
        let mut changed = BTreeSet::new();
        for (index, op) in request.operations.iter().enumerate() {
            candidate.apply(op, &mut changed).map_err(|e| e.at(index))?;
        }
        candidate.prune_links(&mut changed);
        candidate.validate()?;
        metrics.validate_us = us(time);
        let result = BatchResult {
            project_id: self.project_id.clone(),
            committed_revision: before.revision + 1,
            changed_ids: changed.into_iter().collect(),
            render_status: "pending".into(),
            replayed: false,
        };
        self.persist(
            &mut conn,
            &before,
            &candidate,
            &request.idempotency_key,
            &request_hash,
            &request_value["operations"],
            &result,
            &mut metrics,
        )?;
        metrics.total_us = us(total);
        Ok((result, metrics))
    }
    pub(crate) fn request_ids(&self, project: &str, key: &str) -> Result<()> {
        if project != self.project_id {
            return Err(StorageError::validation(
                "projectId",
                "Request project ID does not match this project",
            ));
        }
        if !validate::valid_id(key) {
            return Err(StorageError::validation(
                "idempotencyKey",
                "Idempotency key must contain 1–128 ASCII letters, digits, underscores or hyphens",
            ));
        }
        Ok(())
    }
    pub fn commits_after(&self, revision: u64, limit: usize) -> Result<Vec<Commit>> {
        let mut conn = self.reader()?;
        let tx = conn.transaction()?;
        let meta = read_meta(&tx)?;
        if revision > meta.revision {
            let mut e = StorageError::new(
                "RESET_REQUIRED",
                "Event cursor is ahead of the project; refetch snapshot",
            );
            e.current_revision = Some(meta.revision);
            return Err(e);
        }
        let commits = {
            let mut stmt=tx.prepare("SELECT revision,base_revision,batch_id,ops_json,summary_json FROM commits WHERE revision>?1 ORDER BY revision LIMIT ?2")?;
            let rows =
                stmt.query_map(params![revision as i64, limit.clamp(1, 1000) as i64], |r| {
                    Ok((
                        r.get::<_, i64>(0)? as u64,
                        r.get::<_, i64>(1)? as u64,
                        r.get(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                    ))
                })?;
            rows.map(|r| {
                let (revision, base_revision, batch_id, ops, summary) = r?;
                Ok(Commit {
                    revision,
                    base_revision,
                    batch_id,
                    operations: serde_json::from_str(&ops)?,
                    changed_ids: serde_json::from_str(&summary)?,
                })
            })
            .collect::<Result<Vec<_>>>()?
        };
        tx.commit()?;
        Ok(commits)
    }
    pub fn checkpoint(&self) -> Result<Checkpoint> {
        self.check_paths()?;
        let conn = self.writer.lock().map_err(|_| poisoned())?;
        Ok(conn.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |r| {
            Ok(Checkpoint {
                busy: r.get(0)?,
                wal_pages: r.get(1)?,
                checkpointed_pages: r.get(2)?,
            })
        })?)
    }
}
