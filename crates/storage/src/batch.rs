//! One candidate-validation pipeline for committed and previewed batches.
use crate::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DryRunResult {
    pub project_id: String,
    pub dry_run: bool,
    pub base_revision: u64,
    pub changed_ids: Vec<String>,
    pub diagnostics: Vec<Value>,
}

enum Outcome {
    Committed(BatchResult),
    Preview(DryRunResult),
}

impl Project {
    pub fn apply_batch(&self, request: BatchRequest) -> Result<BatchResult> {
        self.apply_batch_measured(request).map(|(r, _)| r)
    }
    pub fn apply_batch_measured(
        &self,
        request: BatchRequest,
    ) -> Result<(BatchResult, BatchMetrics)> {
        match self.run_batch(request, false, None)? {
            (Outcome::Committed(result), metrics) => Ok((result, metrics)),
            _ => unreachable!("commit mode returns a commit"),
        }
    }
    /// Preview checks existing key collisions without reserving a key. Revision and
    /// full candidate validation still run under the same project writer lock.
    pub fn dry_run_batch(&self, request: BatchRequest) -> Result<DryRunResult> {
        match self.run_batch(request, true, None)? {
            (Outcome::Preview(result), _) => Ok(result),
            _ => unreachable!("preview mode returns a preview"),
        }
    }
    pub fn apply_scoped_batch(&self, request: ScopedBatchRequest) -> Result<BatchResult> {
        let (request, pages) = request.split();
        match self.run_batch(request, false, Some(pages))? {
            (Outcome::Committed(result), _) => Ok(result),
            _ => unreachable!(),
        }
    }
    pub fn dry_run_scoped_batch(&self, request: ScopedBatchRequest) -> Result<DryRunResult> {
        let (request, pages) = request.split();
        match self.run_batch(request, true, Some(pages))? {
            (Outcome::Preview(result), _) => Ok(result),
            _ => unreachable!(),
        }
    }
    fn run_batch(
        &self,
        request: BatchRequest,
        dry_run: bool,
        pages: Option<Vec<String>>,
    ) -> Result<(Outcome, BatchMetrics)> {
        let total = Instant::now();
        self.check_paths()?;
        self.request_ids(&request.project_id, &request.idempotency_key)?;
        if request.operations.is_empty() || request.operations.len() > MAX_BATCH_OPS {
            return Err(StorageError::validation(
                "operations",
                "Batch requires 1–500 operations",
            ));
        }
        let request_value = scoped::request_value(&request, pages.as_deref())?;
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
            if !dry_run {
                result.replayed = true;
                metrics.total_us = us(total);
                return Ok((Outcome::Committed(result), metrics));
            }
        }
        let time = Instant::now();
        let before = read_snapshot(&mut conn)?;
        metrics.read_us = us(time);
        if pages.is_none() && before.revision != request.expected_revision {
            return Err(StorageError::conflict(before.revision));
        }
        let time = Instant::now();
        let (candidate, changed_ids) = validated_candidate(&before, &request.operations)?;
        if let Some(pages) = &pages {
            let commits = {
                let mut stmt = conn.prepare("SELECT revision,CASE WHEN sum(length(CAST(ops_json AS BLOB))) OVER (ORDER BY revision)<=8388608 THEN ops_json ELSE 'null' END FROM commits WHERE revision>?1 ORDER BY revision LIMIT 1001")?;
                let rows = stmt.query_map([request.expected_revision as i64], |r| {
                    Ok((r.get::<_, i64>(0)? as u64, r.get::<_, String>(1)?))
                })?;
                rows.map(|row| {
                    let (revision, text) = row?;
                    Ok((revision, serde_json::from_str(&text)?))
                })
                .collect::<Result<Vec<_>>>()?
            };
            scoped::check(&before, &candidate, &request, pages, commits)?;
        }
        metrics.validate_us = us(time);
        if dry_run {
            metrics.total_us = us(total);
            return Ok((
                Outcome::Preview(DryRunResult {
                    project_id: self.project_id.clone(),
                    dry_run: true,
                    base_revision: before.revision,
                    changed_ids,
                    diagnostics: vec![],
                }),
                metrics,
            ));
        }
        let result = BatchResult {
            project_id: self.project_id.clone(),
            committed_revision: before.revision + 1,
            changed_ids,
            render_status: "pending".into(),
            replayed: false,
        };
        let history = scoped::envelope(
            &before,
            &candidate,
            request_value.clone(),
            request_value["operations"].clone(),
        )?;
        self.persist(
            &mut conn,
            &before,
            &candidate,
            &request.idempotency_key,
            &request_hash,
            &history,
            &result,
            &mut metrics,
        )?;
        metrics.total_us = us(total);
        Ok((Outcome::Committed(result), metrics))
    }
}

pub(crate) fn validated_candidate(
    before: &Snapshot,
    operations: &[Operation],
) -> Result<(Candidate, Vec<String>)> {
    let mut candidate = Candidate::from_snapshot(before);
    let mut changed = BTreeSet::new();
    for (index, op) in operations.iter().enumerate() {
        candidate.apply(op, &mut changed).map_err(|e| e.at(index))?;
    }
    candidate.prune_links(&mut changed);
    candidate.validate()?;
    Ok((candidate, changed.into_iter().collect()))
}
