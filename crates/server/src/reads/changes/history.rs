//! The contiguous run of durable commits after a revision, cut at a snapshot revision.
//! Storage keeps the log for the project lifetime, but a bundle import clears it, so a range
//! can be missing; callers decide whether that is an error or a truncation.
use crate::{error::Result, state::AppState};
use opencanvas_storage::Commit;

pub(super) struct History {
    /// Commits `since+1, since+2, …` in order, none above `upto`.
    pub commits: Vec<Commit>,
    /// The log has no row for the next expected revision at or below `upto`.
    pub gap: bool,
}

/// Reads at most `limit` commits after `since` from the project's own read transaction, keeping
/// only those up to `upto` (the caller's snapshot revision) so the result matches that snapshot
/// even when a commit lands between the two reads.
pub(super) async fn after(
    state: &AppState,
    project: &str,
    since: u64,
    upto: u64,
    limit: usize,
) -> Result<History> {
    if since >= upto {
        return Ok(History {
            commits: Vec::new(),
            gap: false,
        });
    }
    let fetched = state
        .project(project.to_owned(), move |p| p.commits_after(since, limit))
        .await?;
    let full = fetched.len() >= limit;
    let mut commits = Vec::with_capacity(fetched.len());
    let mut gap = false;
    for commit in fetched {
        if commit.revision > upto {
            break;
        }
        if commit.revision != since + commits.len() as u64 + 1 {
            gap = true;
            break;
        }
        commits.push(commit);
    }
    let last = commits.last().map_or(since, |c| c.revision);
    // Fewer rows than asked for, yet short of `upto`: the missing revisions have no log row.
    gap |= !full && last < upto;
    Ok(History { commits, gap })
}
