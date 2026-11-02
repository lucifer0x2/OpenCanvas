//! Opt-in stderr timings; no project IDs, keys, SQL, connection URLs or props.
use oc_spike_core::ApplyProfile;
use serde::Serialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
    time::Instant,
};

static ENABLED: OnceLock<bool> = OnceLock::new();
static SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchEvidence {
    pub path: &'static str,
    pub fallback_reason: Option<&'static str>,
    pub queried_node_count: usize,
    pub validated_node_count: usize,
}

pub struct BatchProfile {
    started: Option<Instant>,
    sequence: u64,
    phases: BTreeMap<&'static str, u64>,
    operations: usize,
    nodes: usize,
    outcome: &'static str,
    pub core: ApplyProfile,
    pub evidence: BatchEvidence,
}

impl BatchProfile {
    pub fn new(operations: usize) -> Self {
        let enabled =
            *ENABLED.get_or_init(|| std::env::var("OC_SPIKE_PROFILE_BATCH").as_deref() == Ok("1"));
        Self {
            started: enabled.then(Instant::now),
            sequence: if enabled {
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            } else {
                0
            },
            phases: BTreeMap::new(),
            operations,
            nodes: 0,
            outcome: "error",
            core: ApplyProfile::default(),
            evidence: BatchEvidence::default(),
        }
    }

    pub fn timer(&self) -> Option<Instant> {
        self.started.map(|_| Instant::now())
    }

    pub fn record(&mut self, phase: &'static str, started: Option<Instant>) {
        if let Some(started) = started {
            self.phases.insert(phase, micros(started));
        }
    }

    pub fn enabled(&self) -> bool {
        self.started.is_some()
    }

    pub fn finish(&mut self, outcome: &'static str, nodes: usize) {
        self.outcome = outcome;
        self.nodes = nodes;
    }
}

impl Drop for BatchProfile {
    fn drop(&mut self) {
        if let Some(started) = self.started {
            eprintln!(
                "{}",
                json!({
                    "batchProfile": true,
                    "profileSequence": self.sequence,
                    "operationCount": self.operations,
                    "nodeCount": self.nodes,
                    "outcome": self.outcome,
                    "totalUs": micros(started),
                    "phasesUs": self.phases,
                    "coreUs": self.core,
                    "writeEvidence": self.evidence,
                })
            );
        }
    }
}

fn micros(started: Instant) -> u64 {
    started.elapsed().as_micros().try_into().unwrap_or(u64::MAX)
}
