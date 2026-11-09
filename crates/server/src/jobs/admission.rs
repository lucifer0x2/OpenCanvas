//! Admit prepared work under one lock, with atomic capacity checks for page batches.
use super::{
    prepared::Prepared,
    queue::{Priority, Work, DIRECT_CAP},
    Entry, Jobs, Table,
};
use crate::error::Result;
use opencanvas_storage::Snapshot;
use serde_json::{json, Value};

impl Jobs {
    /// Every frame validates against the same snapshot before any job is admitted.
    pub(crate) async fn enqueue_batch(
        &self,
        snapshot: &Snapshot,
        requests: Vec<super::SceneRequest>,
    ) -> Result<Vec<Value>> {
        let prepared = requests
            .into_iter()
            .map(|request| Prepared::new(snapshot, request, &self.config, None))
            .collect::<Result<Vec<_>>>()?;
        let mut table = self.table.lock().await;
        let new = prepared
            .iter()
            .filter(|p| table.cached(&p.key, true).is_none())
            .count();
        let promoted = prepared
            .iter()
            .filter_map(|p| table.cached(&p.key, true))
            .filter(|entry| {
                table
                    .queue
                    .is_background(entry.value["jobId"].as_str().unwrap_or_default())
            })
            .count();
        if new > 0 && table.queue.direct_len() + new + promoted > DIRECT_CAP {
            return Err(table.pace.queue_full(
                "Render queue cannot fit this batch; retry later",
                table.queue.direct_len() + 1,
            ));
        }
        let keep: Vec<_> = prepared
            .iter()
            .filter_map(|p| table.cached(&p.key, true))
            .filter_map(|entry| entry.value["jobId"].as_str().map(str::to_owned))
            .collect();
        table.reserve(new, &keep)?;
        let values = prepared
            .into_iter()
            .map(|p| table.admit(p, true, Priority::Direct))
            .collect::<Result<Vec<_>>>()?;
        drop(table);
        let _ = self.doorbell.try_send(());
        Ok(values)
    }
}

#[cfg(test)]
#[path = "admission_tests.rs"]
mod tests;

impl Table {
    fn cached(&self, key: &str, retry: bool) -> Option<&Entry> {
        self.entries.iter().rev().find(|e| {
            e.key == key
                && (!retry || (e.value["status"] != "failed" && e.value.get("error").is_none()))
        })
    }

    pub(super) fn admit(
        &mut self,
        prepared: Prepared,
        retry: bool,
        priority: Priority,
    ) -> Result<Value> {
        let Prepared {
            key,
            input,
            overflow,
            project,
            revision,
            request,
            export,
        } = prepared;
        let direct = priority == Priority::Direct;
        if let Some(entry) = self.entries.iter_mut().rev().find(|e| {
            e.key == key
                && (!retry || (e.value["status"] != "failed" && e.value.get("error").is_none()))
        }) {
            entry.requested |= direct;
            let value = entry.value.clone();
            if direct && value["status"] == "pending" {
                self.queue
                    .promote(value["jobId"].as_str().unwrap_or_default());
            }
            return Ok(value);
        }
        let id = format!("job_{}", uuid::Uuid::new_v4().simple());
        let mut value = json!({"jobId":id,"status":"pending","projectId":project,"revision":revision,"frameId":request.frame_id,"sceneKey":key});
        if let Some(spec) = &export {
            value["kind"] = json!("export");
            value["nodeId"] = json!(spec.node_id);
        }
        let work = Work {
            text_overflow_intent: overflow,
            id,
            key: key.clone(),
            input,
            scale: request.scale,
            frame: (project, request.frame_id),
            export,
        };
        match priority {
            Priority::Direct => {
                self.make_room()?;
                if !self.queue.push_direct(work) {
                    return Err(self.pace.queue_full(
                        "Render queue is full; retry later",
                        self.queue.direct_len() + 1,
                    ));
                }
            }
            Priority::Background => {
                let dropped = self.queue.push_background(work);
                self.forget(&dropped);
                self.make_room()?;
            }
        }
        self.entries.push_back(Entry {
            key,
            value: value.clone(),
            png: None,
            requested: direct,
            files: Vec::new(),
        });
        Ok(value)
    }
}
