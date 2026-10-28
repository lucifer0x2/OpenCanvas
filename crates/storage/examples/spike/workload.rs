use super::{fixture::*, report::*};
use opencanvas_storage::*;
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Barrier,
    },
    time::{Duration, Instant},
};

pub fn sequential(p: &Project, kind: &str) -> Value {
    let mut metrics = Vec::new();
    let mut revision = p.snapshot().unwrap().revision;
    for sample in 0..30 {
        let ops = match kind {
            "create" => (0..10)
                .map(|i| {
                    node(
                        format!("new-{sample}-{i}"),
                        NodeKind::Rectangle,
                        Some("frame-a"),
                        json!({"width":80,"height":40,"fill":"#123456"}),
                    )
                })
                .collect(),
            "update" => updates(&format!("sequential-{sample}")),
            _ => (0..10)
                .map(|i| Operation::ReparentNode {
                    id: format!("leaf-{i}"),
                    parent_id: if sample % 2 == 0 {
                        "frame-b"
                    } else {
                        "frame-a"
                    }
                    .into(),
                    index: None,
                })
                .collect(),
        };
        let (result, m) = p
            .apply_batch_measured(req(p, revision, format!("{kind}-{sample}"), ops))
            .unwrap();
        revision = result.committed_revision;
        metrics.push(m);
        if kind == "create" {
            let cleanup = (0..10)
                .map(|i| Operation::DeleteSubtree {
                    id: format!("new-{sample}-{i}"),
                })
                .collect();
            revision = p
                .apply_batch(req(p, revision, format!("cleanup-{sample}"), cleanup))
                .unwrap()
                .committed_revision;
        }
    }
    json!({"kind":kind,"samples":metrics.len(),"metrics":metrics_summary(&metrics)})
}

pub fn contention(p: &Arc<Project>, clients: usize) -> Value {
    let running = Arc::new(AtomicBool::new(true));
    let barrier = Arc::new(Barrier::new(clients));
    let reader = {
        let p = p.clone();
        let running = running.clone();
        std::thread::spawn(move || {
            let mut read_us = Vec::new();
            let mut serialize_us = Vec::new();
            let mut last = 0;
            while running.load(Ordering::Acquire) {
                let t = Instant::now();
                let s = p.snapshot().unwrap();
                read_us.push(t.elapsed().as_micros() as u64);
                assert!(s.revision >= last);
                last = s.revision;
                assert_coherent(&s);
                let t = Instant::now();
                std::hint::black_box(serde_json::to_vec(&s).unwrap());
                serialize_us.push(t.elapsed().as_micros() as u64);
                std::thread::sleep(Duration::from_millis(5));
            }
            (read_us, serialize_us)
        })
    };
    let handles: Vec<_> = (0..clients)
        .map(|client| {
            let p = p.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut metrics = Vec::new();
                let mut conflicts = 0;
                barrier.wait();
                for sample in 0..20 {
                    let key = format!("clients-{clients}-{client}-{sample}");
                    loop {
                        let revision = p.snapshot().unwrap().revision;
                        match p.apply_batch_measured(req(&p, revision, key.clone(), updates(&key)))
                        {
                            Ok((_, m)) => {
                                metrics.push(m);
                                break;
                            }
                            Err(e) if e.code == "REVISION_CONFLICT" => {
                                conflicts += 1;
                            }
                            Err(e) => panic!("Unexpected storage failure: {e}"),
                        }
                    }
                }
                (metrics, conflicts)
            })
        })
        .collect();
    let mut metrics = Vec::new();
    let mut conflicts = 0;
    for h in handles {
        let (m, c) = h.join().unwrap();
        metrics.extend(m);
        conflicts += c;
    }
    running.store(false, Ordering::Release);
    let (reads, serialization) = reader.join().unwrap();
    json!({"clients":clients,"successfulBatches":metrics.len(),"revisionConflicts":conflicts,"metrics":metrics_summary(&metrics),"readerSamples":reads.len(),"readUs":percentiles(reads),"serializationUs":percentiles(serialization)})
}
