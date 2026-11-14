//! Items 3 and 4: one `update_node` text change per commit, strict then scoped, with
//! the product phases recorded by `bench_phase` (benchmark feature only).
use super::stats::{ms, summary, Plan};
use opencanvas_storage::{bench_phase, BatchRequest, Operation, ProjectHandle, ScopedBatchRequest};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::time::Instant;

/// Blocking: run on a blocking worker. `revision` is the current project revision.
pub fn measure(
    plan: &Plan,
    handle: &ProjectHandle,
    text_id: &str,
    page_id: &str,
    revision: &mut u64,
) -> Value {
    let mut out = json!({});
    for mode in ["strict", "scoped"] {
        let mut totals = Vec::new();
        let mut phases: BTreeMap<&'static str, Vec<f64>> = BTreeMap::new();
        let mut counts: BTreeMap<&'static str, Vec<f64>> = BTreeMap::new();
        for index in 0..plan.total() {
            let operations = vec![Operation::UpdateNode {
                id: text_id.into(),
                props: json!({"text": format!("Task24 {mode} edit {index}")}),
            }];
            let key = format!("task24-{mode}-{index}");
            bench_phase::take();
            bench_phase::take_counts();
            let start = Instant::now();
            let result = if mode == "strict" {
                handle.apply_batch(BatchRequest {
                    project_id: handle.id().into(),
                    expected_revision: *revision,
                    idempotency_key: key,
                    operations,
                })
            } else {
                handle.apply_scoped_batch(ScopedBatchRequest {
                    project_id: handle.id().into(),
                    expected_revision: *revision,
                    idempotency_key: key,
                    page_ids: vec![page_id.into()],
                    operations,
                })
            }
            .unwrap_or_else(|e| panic!("{mode} commit failed: {}", e.code));
            let elapsed = ms(start);
            assert_eq!(result.committed_revision, *revision + 1);
            *revision += 1;
            let recorded = bench_phase::take();
            let recorded_counts = bench_phase::take_counts();
            if plan.measured(index) {
                totals.push(elapsed);
                let mut sums: BTreeMap<&'static str, f64> = BTreeMap::new();
                for (name, us) in recorded {
                    *sums.entry(name).or_default() += us as f64 / 1_000.0;
                }
                for (name, value) in sums {
                    phases.entry(name).or_default().push(value);
                }
                for (name, value) in recorded_counts {
                    counts.entry(name).or_default().push(value as f64);
                }
            }
        }
        let mut breakdown = Map::new();
        for (name, values) in phases {
            breakdown.insert(name.into(), summary(&values));
        }
        let counts: BTreeMap<_, _> = counts
            .into_iter()
            .map(|(name, values)| {
                let mut result = summary(&values);
                result["unit"] = json!("nodes");
                if name == "validation.full_fallback" {
                    result["unit"] = json!("boolean");
                }
                (name, result)
            })
            .collect();
        out[mode] =
            json!({"total": summary(&totals), "phases": breakdown, "validationCounts": counts});
    }
    out
}
