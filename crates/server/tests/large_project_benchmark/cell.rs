//! One benchmark cell (engine × variant × copies) per process, so the wrapper can
//! attribute peak RSS to it. Order: seed, storage reads, HTTP, MCP reads, commits.
use super::{commits, mcp, scale, seed, snapshots, source, stats, Backends};
use opencanvas_storage::{instances, NodeKind};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Instant;

pub struct Cell {
    pub engine: String,
    pub variant: scale::Variant,
    pub copies: usize,
    pub source_dir: PathBuf,
}

impl Cell {
    pub fn from_env() -> Self {
        let var = |key: &str| std::env::var(key).unwrap_or_else(|_| panic!("{key} is required"));
        Self {
            engine: var("TASK24_ENGINE"),
            variant: scale::Variant::parse(&var("TASK24_VARIANT")),
            copies: var("TASK24_COPIES").parse().unwrap(),
            source_dir: PathBuf::from(var("TASK24_SOURCE_DIR")),
        }
    }
}

pub async fn run(cell: Cell) -> Value {
    let plan = stats::Plan::from_env();
    let source = source::load(&cell.source_dir);
    let targets = source::targets(&source.snapshot);
    let scaled = scale::build(&source.snapshot, cell.copies, cell.variant);
    let authored = scaled.nodes.len();
    let (expand, expanded) =
        stats::sample(&plan, |_| instances::expand(&scaled.nodes).unwrap().len());
    let themes: BTreeSet<&str> = scaled
        .tokens
        .iter()
        .filter_map(|t| t.value.get("themes")?.as_object())
        .flat_map(|o| o.keys().map(String::as_str))
        .collect();
    let counts = json!({
        "authoredNodes": authored, "expandedNodes": expanded,
        "pages": scaled.nodes.iter().filter(|n| n.kind == NodeKind::Page).count(),
        "instances": scaled.nodes.iter().filter(|n| n.kind == NodeKind::Instance).count(),
        "tokens": scaled.tokens.len(), "styles": scaled.styles.len(), "fonts": scaled.fonts.len(),
        "assets": scaled.assets.len(), "assetsWithBytes": source.binaries.len(),
        "notes": scaled.notes.len(), "namedThemes": themes.len(),
    });
    let ids: BTreeSet<String> = scaled.nodes.iter().map(|n| n.id.clone()).collect();
    let slug = format!("t24-{}-{:?}-{:02}", cell.engine, cell.variant, cell.copies).to_lowercase();
    let start = Instant::now();
    // The synchronous PostgreSQL client must not run on an async worker.
    let engine = cell.engine.clone();
    let name = slug.clone();
    let (backends, project) = tokio::task::spawn_blocking(move || {
        let seeded = if engine == "sqlite" {
            let root = super::temp_root();
            let project = seed::sqlite(root.path(), &name, &scaled, &source.binaries);
            (Backends::sqlite(root), project)
        } else {
            let url = std::env::var("TASK19_POSTGRES_URL").expect("disposable PostgreSQL URL");
            let project = seed::postgres(&url, &name, &scaled, &source.binaries);
            (Backends::postgres(&url), project)
        };
        drop((scaled, source));
        seeded
    })
    .await
    .unwrap();
    let seed_ms = stats::ms(start);
    let rss_after_seed = stats::rss_kib();

    let blocking = backends.clone();
    let id = project.clone();
    let storage = tokio::task::spawn_blocking(move || {
        let snapshot = blocking.uncached.snapshot(&id).unwrap();
        let stored: BTreeSet<String> = snapshot.nodes.iter().map(|n| n.id.clone()).collect();
        assert_eq!(stored, ids, "seeded rows match the scaled fixture");
        drop(snapshot);
        snapshots::storage(&stats::Plan::from_env(), &blocking, &id)
    })
    .await
    .unwrap();
    eprintln!("Task24 {slug}: storage reads done");
    let http = snapshots::http(&plan, &backends, &project).await;
    eprintln!("Task24 {slug}: HTTP done");
    let reads = mcp::reads(&plan, &backends, &project, &targets).await;
    eprintln!("Task24 {slug}: MCP reads done");
    let blocking = backends.clone();
    let id = project.clone();
    let commits = tokio::task::spawn_blocking(move || {
        let handle = blocking.primary.open_project(&id).unwrap();
        let mut revision = handle.snapshot().unwrap().revision;
        commits::measure(
            &stats::Plan::from_env(),
            &handle,
            &targets.text,
            &targets.page,
            &mut revision,
        )
    })
    .await
    .unwrap();
    eprintln!("Task24 {slug}: commits done");
    json!({
        "engine": cell.engine, "variant": format!("{:?}", cell.variant).to_lowercase(),
        "copies": cell.copies, "warmup": plan.warmup, "samples": plan.samples,
        "counts": counts, "seedMs": seed_ms, "rssAfterSeedKib": rss_after_seed,
        "expandWholeProject": expand, "storage": storage, "http": http,
        "reads": reads, "commits": commits, "rssEndKib": stats::rss_kib(),
    })
}
