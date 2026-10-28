//! Two-tier render queue. Requested renders (`render_frame`, `get_design_context` viewports)
//! always run before commit prerenders, and prerenders never take their slots. A queued prerender
//! is superseded by a newer prerender of the same frame, or displaced when the prerender tier is
//! full; the running job is never interrupted.
use serde_json::Value;
use std::collections::VecDeque;

/// Queued requested renders; one more runs.
pub const DIRECT_CAP: usize = 8;
/// Queued commit prerenders, at most one per frame.
pub const BACKGROUND_CAP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// A caller asked for this scene and may wait on it.
    Direct,
    /// A best-effort prerender after a commit; never awaited.
    Background,
}

pub struct Work {
    pub id: String,
    pub key: String,
    pub input: Value,
    pub scale: f64,
    /// Supersede scope: the prerender of one frame in one project.
    pub frame: (String, String),
}

/// A queued job that left the queue without running, and the job that replaced it, if any.
pub struct Dropped {
    pub id: String,
    pub by: Option<String>,
}

#[derive(Default)]
pub struct Queue {
    direct: VecDeque<Work>,
    background: VecDeque<Work>,
}

impl Queue {
    /// Returns false, dropping the work, when the requested tier is full.
    pub fn push_direct(&mut self, work: Work) -> bool {
        if self.direct.len() >= DIRECT_CAP {
            return false;
        }
        self.direct.push_back(work);
        true
    }
    /// Queues a prerender and returns the queued prerenders it removed: older ones of the same
    /// frame (replaced by this job), then the oldest of other frames while the tier is over cap.
    pub fn push_background(&mut self, work: Work) -> Vec<Dropped> {
        let mut dropped = Vec::new();
        let mut kept = VecDeque::with_capacity(self.background.len() + 1);
        for queued in self.background.drain(..) {
            if queued.frame == work.frame {
                dropped.push(Dropped {
                    id: queued.id,
                    by: Some(work.id.clone()),
                });
            } else {
                kept.push_back(queued);
            }
        }
        self.background = kept;
        while self.background.len() >= BACKGROUND_CAP {
            let oldest = self.background.pop_front().expect("tier is non-empty");
            dropped.push(Dropped {
                id: oldest.id,
                by: None,
            });
        }
        self.background.push_back(work);
        dropped
    }
    /// Moves a queued prerender into the requested tier, so a caller who now waits on it is not
    /// behind other prerenders and the job can no longer be superseded. Returns false when the
    /// job is not a queued prerender (already requested, running or finished).
    pub fn promote(&mut self, id: &str) -> bool {
        let Some(index) = self.background.iter().position(|w| w.id == id) else {
            return false;
        };
        let work = self.background.remove(index).expect("index is in range");
        self.direct.push_back(work);
        true
    }
    pub fn pop(&mut self) -> Option<Work> {
        self.direct
            .pop_front()
            .or_else(|| self.background.pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn work(id: &str, frame: &str) -> Work {
        Work {
            id: id.into(),
            key: id.into(),
            input: json!({}),
            scale: 1.0,
            frame: ("p".into(), frame.into()),
        }
    }
    fn order(queue: &mut Queue) -> Vec<String> {
        std::iter::from_fn(|| queue.pop()).map(|w| w.id).collect()
    }

    #[test]
    fn requested_renders_run_first_and_ignore_a_full_prerender_tier() {
        let mut queue = Queue::default();
        for i in 0..20 {
            queue.push_background(work(&format!("bg{i}"), &format!("f{i}")));
        }
        assert!(queue.push_direct(work("direct", "f0")));
        let ran = order(&mut queue);
        assert_eq!(ran[0], "direct");
        assert_eq!(ran.len(), 1 + BACKGROUND_CAP);
        assert_eq!(ran.last().unwrap(), "bg19");
    }

    #[test]
    fn newer_prerender_of_a_frame_supersedes_the_queued_one() {
        let mut queue = Queue::default();
        assert!(queue.push_background(work("r1", "a")).is_empty());
        assert!(queue.push_background(work("other", "b")).is_empty());
        let dropped = queue.push_background(work("r2", "a"));
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].id, "r1");
        assert_eq!(dropped[0].by.as_deref(), Some("r2"));
        assert_eq!(order(&mut queue), ["other", "r2"]);
    }

    #[test]
    fn full_prerender_tier_displaces_its_oldest_job() {
        let mut queue = Queue::default();
        for i in 0..BACKGROUND_CAP {
            queue.push_background(work(&format!("bg{i}"), &format!("f{i}")));
        }
        let dropped = queue.push_background(work("new", "fresh"));
        assert_eq!(dropped.len(), 1);
        assert_eq!(
            (dropped[0].id.as_str(), dropped[0].by.as_deref()),
            ("bg0", None)
        );
    }

    #[test]
    fn promoted_prerender_is_not_superseded_and_direct_tier_is_capped() {
        let mut queue = Queue::default();
        queue.push_background(work("bg", "a"));
        queue.push_background(work("bg-b", "b"));
        assert!(queue.promote("bg"));
        assert!(!queue.promote("bg"));
        assert!(queue.push_background(work("bg2", "a")).is_empty());
        assert_eq!(order(&mut queue), ["bg", "bg-b", "bg2"]);
        for i in 0..DIRECT_CAP {
            assert!(queue.push_direct(work(&format!("d{i}"), "a")));
        }
        assert!(!queue.push_direct(work("over", "a")));
    }
}
