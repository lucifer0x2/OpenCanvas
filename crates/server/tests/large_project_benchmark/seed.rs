//! Fixture loading into disposable storage only. The project is created through the
//! product API; its rows are then written directly so a 200k-node project does not
//! take hundreds of whole-project validations to seed. Full fixture validation is
//! explicit and precedes all timed operations (incremental commits require it).
use opencanvas_storage::{PostgresBackend, Snapshot, Storage, Store};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

const NAME: &str = "Task24 benchmark";
const CHUNK: usize = 5_000;

fn json(value: &impl Serialize) -> String {
    serde_json::to_string(value).unwrap()
}

/// Returns the canonical project ID. The store is closed again before returning.
pub fn sqlite(
    root: &Path,
    slug: &str,
    scaled: &Snapshot,
    bins: &BTreeMap<String, Vec<u8>>,
) -> String {
    super::parity::validate_fixture(scaled);
    let id = {
        let storage = Storage::from(Store::open(root).unwrap());
        let handle = storage.create_project(slug, NAME).unwrap();
        handle.id().to_string()
    };
    let dir = root.join(slug);
    for asset in &scaled.assets {
        if let Some(bytes) = bins.get(&asset.content_hash) {
            std::fs::write(dir.join("assets").join(&asset.relative_key), bytes).unwrap();
        }
    }
    let mut conn = rusqlite::Connection::open(dir.join("db.sql")).unwrap();
    let tx = conn.transaction().unwrap();
    tx.execute_batch("DELETE FROM nodes; DELETE FROM tokens; DELETE FROM styles; DELETE FROM assets; DELETE FROM fonts; DELETE FROM notes;").unwrap();
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO nodes(id,parent_id,sort_key,kind,props_json) VALUES(?1,?2,?3,?4,?5)",
            )
            .unwrap();
        for n in &scaled.nodes {
            let props = json(&n.props);
            stmt.execute(rusqlite::params![
                n.id,
                n.parent_id,
                n.sort_key,
                n.kind.as_str(),
                props
            ])
            .unwrap();
        }
        let rows = |table: &str, items: Vec<(String, String)>| {
            let sql = format!("INSERT INTO {table}(id,data_json) VALUES(?1,?2)");
            let mut stmt = tx.prepare(&sql).unwrap();
            for (id, data) in items {
                stmt.execute([id, data]).unwrap();
            }
        };
        rows(
            "tokens",
            scaled
                .tokens
                .iter()
                .map(|t| (t.id.clone(), json(t)))
                .collect(),
        );
        rows(
            "styles",
            scaled
                .styles
                .iter()
                .map(|s| (s.id.clone(), json(s)))
                .collect(),
        );
        rows(
            "fonts",
            scaled
                .fonts
                .iter()
                .map(|f| (f.id.clone(), json(f)))
                .collect(),
        );
        rows(
            "notes",
            scaled
                .notes
                .iter()
                .map(|n| (n.id.clone(), json(n)))
                .collect(),
        );
        let mut stmt = tx
            .prepare(
                "INSERT INTO assets(id,content_hash,relative_key,data_json) VALUES(?1,?2,?3,?4)",
            )
            .unwrap();
        for a in &scaled.assets {
            stmt.execute([&a.id, &a.content_hash, &a.relative_key, &json(a)])
                .unwrap();
        }
    }
    tx.execute(
        "UPDATE project_meta SET current_revision=1 WHERE singleton=1",
        [],
    )
    .unwrap();
    tx.commit().unwrap();
    id
}

pub fn postgres(
    url: &str,
    slug: &str,
    scaled: &Snapshot,
    bins: &BTreeMap<String, Vec<u8>>,
) -> String {
    super::parity::validate_fixture(scaled);
    let id = {
        let storage = Storage::new(PostgresBackend::connect(url).unwrap());
        let handle = storage.create_project(slug, NAME).unwrap();
        handle.id().to_string()
    };
    let mut client = postgres::Client::connect(url, postgres::NoTls).unwrap();
    let mut tx = client.transaction().unwrap();
    tx.batch_execute("SET LOCAL search_path=opencanvas_runtime_v1")
        .unwrap();
    tx.execute("DELETE FROM entities WHERE project_id=$1", &[&id])
        .unwrap();
    let mut rows: Vec<(&str, String, String)> = Vec::new();
    rows.extend(scaled.nodes.iter().map(|n| ("node", n.id.clone(), json(n))));
    rows.extend(
        scaled
            .tokens
            .iter()
            .map(|t| ("token", t.id.clone(), json(t))),
    );
    rows.extend(
        scaled
            .styles
            .iter()
            .map(|s| ("style", s.id.clone(), json(s))),
    );
    rows.extend(
        scaled
            .assets
            .iter()
            .map(|a| ("asset", a.id.clone(), json(a))),
    );
    rows.extend(scaled.fonts.iter().map(|f| ("font", f.id.clone(), json(f))));
    rows.extend(scaled.notes.iter().map(|n| ("note", n.id.clone(), json(n))));
    let insert = "INSERT INTO entities(project_id,kind,id,data) SELECT $1,unnest($2::text[]),unnest($3::text[]),unnest($4::text[])";
    for chunk in rows.chunks(CHUNK) {
        let kinds: Vec<&str> = chunk.iter().map(|r| r.0).collect();
        let ids: Vec<&str> = chunk.iter().map(|r| r.1.as_str()).collect();
        let data: Vec<&str> = chunk.iter().map(|r| r.2.as_str()).collect();
        tx.execute(insert, &[&id, &kinds, &ids, &data]).unwrap();
    }
    for (hash, bytes) in bins {
        if scaled.assets.iter().any(|a| &a.content_hash == hash) {
            tx.execute(
                "INSERT INTO binaries(project_id,content_hash,bytes) VALUES($1,$2,$3) ON CONFLICT DO NOTHING",
                &[&id, hash, bytes],
            )
            .unwrap();
        }
    }
    tx.execute("UPDATE projects SET revision=1 WHERE project_id=$1", &[&id])
        .unwrap();
    tx.commit().unwrap();
    id
}
