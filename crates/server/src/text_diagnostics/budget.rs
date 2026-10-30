//! Fits measured overflow advisories into a read's byte share. Advisory evidence never
//! makes a read fail on its own: it is compacted in fixed, deterministic steps instead,
//! and every step leaves an omission summary that says what was dropped and where to
//! read it in full.
use serde_json::{json, Value};

pub(crate) const ITEM: &str = "TEXT_CONTENT_OUTSIDE_BOX";
pub(crate) const SUMMARY: &str = "TEXT_OVERFLOW_SUMMARY";
/// Measured advisories may use at most this fraction (1/n) of a read's byte budget.
pub(crate) const BUDGET_SHARE: usize = 4;
const SCOPE: &str = "complete-measured-scene-before-pagination";
/// Fits the single-summary step in the 256-byte share of the minimum 1024-byte budget.
const RETRIEVE: &str = "All items: inspect_layout per job, or a larger budget";
/// Item fields kept once examples, box, font and message are dropped.
const COMPACT_FIELDS: [&str; 7] = [
    "code",
    "severity",
    "nodeId",
    "sourceSceneKey",
    "intentionalOverflow",
    "outsideLineCount",
    "inkEvidence",
];

pub(crate) fn is_overflow(issue: &Value) -> bool {
    matches!(issue["code"].as_str(), Some(ITEM | SUMMARY))
}

/// `groups` holds the overflow diagnostics of one job each, in request order. Steps, the
/// first that fits wins: full; per-job provenance dropped (items keep `sourceSceneKey`);
/// item evidence reduced to counts; fewest items dropped, round-robin across jobs; one
/// summary for the whole read.
pub(crate) fn fit(groups: Vec<Vec<Value>>, allowance: usize) -> Vec<Value> {
    let full: Vec<Value> = groups.iter().flatten().cloned().collect();
    if size(&full) <= allowance {
        return full;
    }
    let groups: Vec<Group> = groups.into_iter().filter_map(Group::new).collect();
    let all: Vec<usize> = groups.iter().map(|g| g.items.len()).collect();
    for step in [Step::Provenance, Step::Evidence] {
        let fitted = render(&groups, step, &all);
        if size(&fitted) <= allowance {
            return fitted;
        }
    }
    let total: usize = all.iter().sum();
    for kept in (0..total).rev() {
        let fitted = render(&groups, Step::Nodes, &round_robin(&all, kept));
        if size(&fitted) <= allowance {
            return fitted;
        }
    }
    let omitted: u64 = groups
        .iter()
        .map(|g| g.items.len() as u64 + g.summary["omittedNodes"].as_u64().unwrap_or(0))
        .sum();
    vec![
        json!({"code":SUMMARY,"scope":SCOPE,"reportedNodes":0,"omittedNodes":omitted,
        "compaction":{"step":"summary","droppedNodes":total,"retrieve":RETRIEVE}}),
    ]
}

/// Replaces the overflow entries of one job's diagnostics with their fitted form, at the
/// position of the first one, when they exceed `allowance`; other diagnostics are untouched.
pub(crate) fn fit_in_place(diagnostics: &mut Vec<Value>, allowance: usize) {
    let Some(at) = diagnostics.iter().position(is_overflow) else {
        return;
    };
    let overflow: Vec<Value> = diagnostics
        .iter()
        .filter(|d| is_overflow(d))
        .cloned()
        .collect();
    if size(&overflow) <= allowance {
        return;
    }
    diagnostics.retain(|d| !is_overflow(d));
    let fitted = fit(vec![overflow], allowance);
    diagnostics.splice(at..at, fitted);
}

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Provenance,
    Evidence,
    Nodes,
}

struct Group {
    items: Vec<Value>,
    summary: Value,
}

impl Group {
    fn new(issues: Vec<Value>) -> Option<Self> {
        let (summaries, items): (Vec<_>, Vec<_>) =
            issues.into_iter().partition(|d| d["code"] == SUMMARY);
        let summary = summaries.into_iter().next().unwrap_or_else(|| {
            json!({"code":SUMMARY,"scope":SCOPE,"sourceSceneKey":items.first().map(|i| i["sourceSceneKey"].clone()),
                "reportedNodes":items.len(),"omittedNodes":0})
        });
        (!items.is_empty() || summary["omittedNodes"].as_u64() > Some(0))
            .then_some(Self { items, summary })
    }
}

fn render(groups: &[Group], step: Step, keep: &[usize]) -> Vec<Value> {
    let mut out = Vec::new();
    for (group, &kept) in groups.iter().zip(keep) {
        out.extend(group.items[..kept].iter().map(|item| {
            if step == Step::Provenance {
                return item.clone();
            }
            COMPACT_FIELDS
                .iter()
                .filter_map(|k| Some((k.to_string(), item.get(*k)?.clone())))
                .collect::<serde_json::Map<_, _>>()
                .into()
        }));
        let dropped = group.items.len() - kept;
        let mut summary = group.summary.clone();
        summary.as_object_mut().map(|o| o.remove("provenance"));
        summary["reportedNodes"] = json!(kept);
        summary["omittedNodes"] =
            json!(summary["omittedNodes"].as_u64().unwrap_or(0) + dropped as u64);
        let name = match step {
            Step::Provenance => "provenance",
            Step::Evidence => "evidence",
            Step::Nodes => "nodes",
        };
        summary["compaction"] = json!({"step":name,"droppedNodes":dropped,"retrieve":RETRIEVE});
        out.push(summary);
    }
    out
}

/// Keeps `kept` items in total, taking one per job per round so every job stays represented.
fn round_robin(lengths: &[usize], mut kept: usize) -> Vec<usize> {
    let mut keep = vec![0; lengths.len()];
    let mut round = 0;
    while kept > 0 {
        for (slot, len) in keep.iter_mut().zip(lengths) {
            if kept > 0 && round < *len {
                *slot += 1;
                kept -= 1;
            }
        }
        round += 1;
    }
    keep
}

fn size(issues: &[Value]) -> usize {
    serde_json::to_vec(issues).map_or(usize::MAX, |b| b.len())
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
