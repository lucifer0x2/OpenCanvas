//! Candidate construction and revision-indexed validation, shared by both engines.
use crate::*;
#[cfg(test)]
pub(crate) fn validated_candidate(
    before: &Snapshot,
    operations: &[Operation],
) -> Result<(Candidate, Vec<String>, validate::ValidationWork)> {
    validated_candidate_indexed(
        before,
        operations,
        Arc::new(RevisionIndexes::new(&before.nodes)),
    )
    .map(|(candidate, ids, work, _)| (candidate, ids, work))
}

pub(crate) fn validated_candidate_indexed(
    before: &Snapshot,
    operations: &[Operation],
    indexes: Arc<RevisionIndexes>,
) -> Result<(
    Candidate,
    Vec<String>,
    validate::ValidationWork,
    Arc<RevisionIndexes>,
)> {
    let mut candidate = bench_phase::time("candidate.from_snapshot", || {
        Candidate::from_snapshot(before)
    });
    let mut changed = BTreeSet::new();
    bench_phase::time("candidate.apply", || -> Result<()> {
        for (index, op) in operations.iter().enumerate() {
            candidate.apply(op, &mut changed).map_err(|e| e.at(index))?;
        }
        candidate.prune_links(&mut changed);
        Ok(())
    })?;
    // Stored revisions are validated at every write boundary. Raw imports and
    // bundles retain full validation; benchmark seeders must validate before use.
    let next_indexes = RevisionIndexes::candidate(&indexes, &candidate, operations, &changed);
    let validation = bench_phase::time("candidate.validate", || {
        candidate.validate_after_indexed(
            before,
            operations,
            &changed,
            indexes.topology.clone(),
            next_indexes.topology.clone(),
        )
    });
    #[cfg(debug_assertions)]
    {
        // Every existing storage validation fixture (including rejected batches)
        // exercises the full reference too, comparing complete public errors.
        // The incremental contract requires a fully valid `before`; rows written
        // behind the current validator may legitimately diverge, so only a valid
        // before turns a divergence into a failure.
        let public = |result: Result<()>| result.map_err(|e| serde_json::to_value(e).unwrap());
        let reference = public(candidate.validate());
        let incremental = public(validation.as_ref().map(|_| ()).map_err(Clone::clone));
        if incremental != reference && Candidate::from_snapshot(before).validate().is_ok() {
            panic!(
                "incremental/full candidate validation diverged\n  left: {incremental:?}\n right: {reference:?}"
            );
        }
    }
    let work = validation?;
    bench_phase::count("validation.authored", work.stats.authored_nodes);
    bench_phase::count("validation.affected", work.stats.affected_nodes);
    bench_phase::count("validation.expanded", work.stats.expanded_nodes);
    bench_phase::count(
        "validation.virtual_affected",
        work.stats.affected_virtual_nodes,
    );
    bench_phase::count(
        "validation.full_fallback",
        usize::from(work.stats.full_fallback),
    );
    Ok((candidate, changed.into_iter().collect(), work, next_indexes))
}
