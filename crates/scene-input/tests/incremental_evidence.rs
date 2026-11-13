//! Synthetic 1/15/30-page validation cost, independent of filesystem/database.
#![cfg(feature = "large-project-bench")]
mod incremental_support;
use incremental_support::*;
use opencanvas_scene_input::{bench_phase, validate::Candidate};
use serde_json::json;
use std::{collections::BTreeSet, time::Instant};

#[test]
#[ignore = "release large-project validation evidence; no production caps changed"]
fn scaled_validation_evidence() {
    assert!(!cfg!(debug_assertions));
    for copies in [1, 15, 30] {
        // ~201k authored nodes at 30x, with nested instances, overrides, shared
        // masters/resources and themes; full-valid fixture outside the timings.
        let before = fixture(copies, 6700);
        let ops = [update("text0", json!({"text":"Changed"}))];
        let mut candidate = Candidate::from_snapshot(&before);
        let mut changed = BTreeSet::new();
        candidate.apply(&ops[0], &mut changed).unwrap();
        let mut full = Vec::new();
        let mut incremental = Vec::new();
        let mut last = None;
        for sample in 0..6 {
            bench_phase::take();
            let start = Instant::now();
            candidate.validate().unwrap();
            let elapsed = start.elapsed().as_secs_f64() * 1000.;
            if sample > 0 {
                full.push(elapsed);
            }
            bench_phase::take();
            let start = Instant::now();
            let work = candidate.validate_after(&before, &ops, &changed).unwrap();
            let elapsed = start.elapsed().as_secs_f64() * 1000.;
            if sample > 0 {
                incremental.push(elapsed);
                eprintln!(
                    "copies={copies} sample={sample} phases={:?}",
                    bench_phase::take()
                );
            }
            last = Some(work.stats);
        }
        // Differential failures and successes on the identical scaled fixture.
        parity(&before, &ops).unwrap();
        assert!(parity(&before, &[update("group0", json!({"width":"hug"}))]).is_err());
        assert!(parity(
            &before,
            &[update(
                "instance0",
                json!({"overrides":{"nested":{"opacity":2}}})
            )]
        )
        .is_err());
        let master = parity(
            &before,
            &[update("label", json!({"text":"Changed master"}))],
        )
        .unwrap();
        assert_eq!(master.stats.affected_virtual_nodes, 1 + 2 * copies);
        full.sort_by(f64::total_cmp);
        incremental.sort_by(f64::total_cmp);
        eprintln!("RESULT copies={copies} counts={:?} full_median_ms={:.3} full_p95_ms={:.3} incremental_median_ms={:.3} incremental_p95_ms={:.3}",
            last.unwrap(),full[2],full[4],incremental[2],incremental[4]);
    }
}
