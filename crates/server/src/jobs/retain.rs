//! Which job records the 32-record table keeps. A record matters while someone may still read it
//! by job ID: a requested render (`render_frame`, context viewports, HTTP scene), or a prerender a
//! requested render reused. A queued prerender that leaves the queue without running has no such
//! reader, so it is forgotten at once rather than kept as a failed record.
use super::{Entry, Table};
use crate::error::Result;

/// Retained job records, pending ones included.
pub const RECORD_CAP: usize = 32;

impl Table {
    /// Reserve the entire batch before eviction, keeping every cache hit the batch returns.
    pub(super) fn reserve(&mut self, count: usize, keep: &[String]) -> Result<()> {
        let needed = (self.entries.len() + count).saturating_sub(RECORD_CAP);
        let removable = |e: &&Entry| {
            e.value["status"] != "pending" && !keep.iter().any(|id| e.value["jobId"] == id.as_str())
        };
        let candidates: Vec<_> = self
            .entries
            .iter()
            .filter(removable)
            .filter(|e| !e.requested)
            .chain(
                self.entries
                    .iter()
                    .filter(removable)
                    .filter(|e| e.requested),
            )
            .take(needed)
            .map(|e| e.value["jobId"].clone())
            .collect();
        if candidates.len() < needed {
            return Err(self
                .pace
                .queue_full("Render job capacity reached", self.entries.len()));
        }
        // Job IDs distinguish a failed attempt and its successful retry with the same key.
        let mut remaining = needed;
        self.entries.retain(|entry| {
            if remaining > 0 && candidates.contains(&entry.value["jobId"]) {
                remaining -= 1;
                false
            } else {
                true
            }
        });
        Ok(())
    }
    /// Drops records until one more fits: finished prerenders nobody requested first, oldest
    /// first, then the oldest finished record of any kind. Fails with `QUEUE_FULL` when every
    /// retained record is still pending.
    pub(super) fn make_room(&mut self) -> Result<()> {
        while self.entries.len() >= RECORD_CAP {
            let finished = |e: &Entry| e.value["status"] != "pending";
            let index = self
                .entries
                .iter()
                .position(|e| finished(e) && !e.requested)
                .or_else(|| self.entries.iter().position(finished));
            let Some(index) = index else {
                let ahead = self.entries.len();
                return Err(self.pace.queue_full("Render job capacity reached", ahead));
            };
            self.entries.remove(index);
        }
        Ok(())
    }
    /// Removes the records of queued prerenders that left the queue without running.
    pub(super) fn forget(&mut self, dropped: &[String]) {
        self.entries
            .retain(|e| !dropped.iter().any(|id| e.value["jobId"] == id.as_str()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entry(id: &str, status: &str, requested: bool) -> Entry {
        Entry {
            key: id.into(),
            value: json!({"jobId":id,"status":status}),
            png: None,
            requested,
            files: Vec::new(),
        }
    }
    fn ids(table: &Table) -> Vec<String> {
        let id = |e: &Entry| e.value["jobId"].as_str().unwrap().to_owned();
        table.entries.iter().map(id).collect()
    }

    #[test]
    fn full_table_drops_unrequested_prerenders_before_requested_records() {
        let mut table = Table::default();
        table.entries.push_back(entry("asked", "ready", true));
        for i in 1..RECORD_CAP {
            table
                .entries
                .push_back(entry(&format!("pre{i}"), "ready", false));
        }
        table.make_room().unwrap();
        assert_eq!(table.entries.len(), RECORD_CAP - 1);
        assert_eq!(ids(&table)[..2], ["asked", "pre2"]);
    }

    #[test]
    fn requested_records_go_oldest_first_and_pending_ones_stay() {
        let mut table = Table::default();
        table.entries.push_back(entry("running", "pending", false));
        for i in 1..RECORD_CAP {
            table
                .entries
                .push_back(entry(&format!("asked{i}"), "failed", true));
        }
        table.make_room().unwrap();
        assert_eq!(ids(&table)[..2], ["running", "asked2"]);
    }

    #[test]
    fn all_pending_reports_queue_full_scaled_by_jobs_ahead() {
        let mut table = Table::default();
        for i in 0..RECORD_CAP {
            table
                .entries
                .push_back(entry(&format!("p{i}"), "pending", true));
        }
        let error = table.make_room().unwrap_err().value();
        assert_eq!(error["error"]["code"], "QUEUE_FULL");
        assert_eq!(error["error"]["retryAfterMs"], 10_000);
        table.forget(&["p0".into(), "p5".into()]);
        assert_eq!(table.entries.len(), RECORD_CAP - 2);
        table.make_room().unwrap();
    }

    #[test]
    fn batch_reservation_keeps_cache_hits_and_failed_reservation_evicts_nothing() {
        let mut table = Table::default();
        table.entries.push_back(entry("cached", "ready", true));
        table.entries.push_back(entry("discard", "ready", false));
        for i in 2..RECORD_CAP {
            table
                .entries
                .push_back(entry(&format!("pending{i}"), "pending", true));
        }
        let before = ids(&table);
        let error = table.reserve(2, &["cached".into()]).unwrap_err();
        assert_eq!(error.value()["error"]["code"], "QUEUE_FULL");
        assert_eq!(ids(&table), before);
        table.reserve(1, &["cached".into()]).unwrap();
        assert_eq!(table.entries.len(), RECORD_CAP - 1);
        assert!(ids(&table).contains(&"cached".to_owned()));
        assert!(!ids(&table).contains(&"discard".to_owned()));
    }
}
