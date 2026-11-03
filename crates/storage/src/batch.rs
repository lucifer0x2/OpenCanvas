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
        match self.run_batch(request, false)? {
            (Outcome::Committed(result), metrics) => Ok((result, metrics)),
            _ => unreachable!("commit mode returns a commit"),
        }
    }
    /// Preview checks existing key collisions without reserving a key. Revision and
    /// full candidate validation still run under the same project writer lock.
    pub fn dry_run_batch(&self, request: BatchRequest) -> Result<DryRunResult> {
        match self.run_batch(request, true)? {
            (Outcome::Preview(result), _) => Ok(result),
            _ => unreachable!("preview mode returns a preview"),
        }
    }
    fn run_batch(&self, request: BatchRequest, dry_run: bool) -> Result<(Outcome, BatchMetrics)> {
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
            if !dry_run {
                result.replayed = true;
                metrics.total_us = us(total);
                return Ok((Outcome::Committed(result), metrics));
            }
        }
        let time = Instant::now();
        let before = read_snapshot(&mut conn)?;
        metrics.read_us = us(time);
        if before.revision != request.expected_revision {
            return Err(StorageError::conflict(before.revision));
        }
        let time = Instant::now();
        let (candidate, changed_ids) = validated_candidate(&before, &request.operations)?;
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
