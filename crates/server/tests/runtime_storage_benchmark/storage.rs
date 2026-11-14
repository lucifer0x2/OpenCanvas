use super::{
    fixture::{self, Project},
    hot_proof,
    metrics::{elapsed_us, Samples, SAMPLES, WARMUP},
};
use opencanvas_storage::{Operation, Snapshot};
use serde_json::{json, Value};
use std::time::Instant;

pub struct Measured {
    pub project: Project,
    pub snapshot: Snapshot,
    pub commit: Samples,
    pub direct: Samples,
    pub hot_proof: Value,
    pub first_post_write_read_us: f64,
    pub seed_revision: u64,
}

pub fn measure(project: Project) -> Measured {
    let mut writes = Vec::new();
    let mut first_measured = None;
    let mut first_result = None;
    for index in 0..WARMUP + SAMPLES {
        let operations = (2..5)
            .map(|id| Operation::UpdateNode {
                merge: false,
                id: format!("node-{id:05}"),
                props: json!({"x":1000+index}),
            })
            .collect();
        let request = fixture::request(
            project.handle.id(),
            project.seed_revision + index as u64,
            &format!("write-{index:04}"),
            operations,
        );
        if index == WARMUP {
            first_measured = Some(request.clone());
        }
        let start = Instant::now();
        let result = project.handle.apply_batch(request).unwrap();
        let elapsed = elapsed_us(start);
        assert_eq!(
            result.committed_revision,
            project.seed_revision + index as u64 + 1
        );
        assert!(!result.replayed);
        assert_eq!(
            result.changed_ids,
            ["node-00002", "node-00003", "node-00004"]
        );
        if index == WARMUP {
            first_result = Some(result.clone());
        }
        if index >= WARMUP {
            writes.push(elapsed);
        }
    }
    // Harness verification/serialization/counters are outside timers. Backend work is included.
    let start = Instant::now();
    let snapshot = project.handle.snapshot().unwrap();
    let first_post_write_read_us = elapsed_us(start);
    verify(&project, &snapshot);
    let before = hot_proof::counters(project.moka);
    let mut reads = Vec::new();
    for index in 0..WARMUP + SAMPLES {
        let start = Instant::now();
        let actual = project.handle.snapshot().unwrap();
        let elapsed = elapsed_us(start);
        assert_eq!(actual, snapshot);
        if index >= WARMUP {
            reads.push(elapsed);
        }
    }
    let hot_proof = hot_proof::verify(before, WARMUP + SAMPLES);
    verify(&project, &snapshot);
    let history_before =
        serde_json::to_value(project.handle.commits_after(0, 1000).unwrap()).unwrap();
    let replay = project.handle.apply_batch(first_measured.unwrap()).unwrap();
    assert!(replay.replayed);
    let mut expected_replay = first_result.unwrap();
    expected_replay.replayed = true;
    assert_eq!(replay, expected_replay);
    assert_eq!(
        replay.committed_revision,
        project.seed_revision + WARMUP as u64 + 1
    );
    assert_eq!(project.handle.snapshot().unwrap(), snapshot);
    assert_eq!(
        serde_json::to_value(project.handle.commits_after(0, 1000).unwrap()).unwrap(),
        history_before
    );
    let seed_revision = project.seed_revision;
    Measured {
        project,
        snapshot,
        commit: Samples::new(writes),
        direct: Samples::new(reads),
        hot_proof,
        first_post_write_read_us,
        seed_revision,
    }
}

pub fn verify(project: &Project, snapshot: &Snapshot) {
    assert_eq!(snapshot.nodes.len(), project.count);
    assert_eq!(
        snapshot.revision,
        project.seed_revision + (WARMUP + SAMPLES) as u64
    );
    for id in 2..5 {
        let node = snapshot
            .nodes
            .iter()
            .find(|n| n.id == format!("node-{id:05}"))
            .unwrap();
        assert_eq!(node.props["x"], 1000 + WARMUP + SAMPLES - 1);
    }
    let history = project.handle.commits_after(0, 1000).unwrap();
    assert_eq!(history.len(), snapshot.revision as usize);
    for (index, commit) in history.iter().enumerate() {
        assert_eq!(commit.revision, index as u64 + 1);
        assert_eq!(commit.base_revision, index as u64);
        if index >= project.seed_revision as usize {
            assert_eq!(
                commit.changed_ids,
                ["node-00002", "node-00003", "node-00004"]
            );
            assert_eq!(commit.operations.as_array().unwrap().len(), 3);
            for operation in commit.operations.as_array().unwrap() {
                assert_eq!(
                    operation["props"]["x"],
                    1000 + index - project.seed_revision as usize
                );
            }
        }
    }
}
