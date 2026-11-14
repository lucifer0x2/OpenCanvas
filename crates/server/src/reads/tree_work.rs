//! Test-only record of the collections one scoped tree read processed, so
//! bounds tests can observe server-layer work without timing.
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TreeWork {
    pub authored: usize,
    pub dependencies: usize,
    pub expanded: usize,
    pub resolved: usize,
    pub notes: usize,
    pub ordered: usize,
    pub entries: usize,
}

thread_local! {
    static LAST: Cell<TreeWork> = Cell::default();
}

pub(super) fn record(work: TreeWork) {
    LAST.with(|last| last.set(work));
}

pub(super) fn last() -> TreeWork {
    LAST.with(Cell::get)
}
