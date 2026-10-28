use crate::*;
pub(crate) fn initialize(conn: &mut Connection, id: &str, slug: &str, name: &str) -> Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute_batch("CREATE TABLE project_meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1),project_id TEXT NOT NULL UNIQUE,slug TEXT NOT NULL,name TEXT NOT NULL,schema_version INTEGER NOT NULL,current_revision INTEGER NOT NULL,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
        CREATE TABLE nodes(id TEXT PRIMARY KEY,parent_id TEXT REFERENCES nodes(id) DEFERRABLE INITIALLY DEFERRED,sort_key INTEGER NOT NULL,kind TEXT NOT NULL,props_json TEXT NOT NULL);
        CREATE INDEX nodes_children ON nodes(parent_id,sort_key,id);
        CREATE TABLE tokens(id TEXT PRIMARY KEY,data_json TEXT NOT NULL);
        CREATE TABLE styles(id TEXT PRIMARY KEY,data_json TEXT NOT NULL);
        CREATE TABLE assets(id TEXT PRIMARY KEY,content_hash TEXT NOT NULL,relative_key TEXT NOT NULL UNIQUE,data_json TEXT NOT NULL);
        CREATE TABLE fonts(id TEXT PRIMARY KEY,data_json TEXT NOT NULL);
        CREATE TABLE commits(revision INTEGER PRIMARY KEY,base_revision INTEGER NOT NULL,batch_id TEXT NOT NULL,ops_json TEXT NOT NULL,summary_json TEXT NOT NULL,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
        CREATE TABLE idempotency(key TEXT PRIMARY KEY,request_hash TEXT NOT NULL,committed_revision INTEGER NOT NULL,response_json TEXT NOT NULL);
        PRAGMA user_version=1;")?;
    tx.execute("INSERT INTO project_meta(singleton,project_id,slug,name,schema_version,current_revision) VALUES(1,?1,?2,?3,1,0)",params![id,slug,name])?;
    let font = Font {
        id: "font_bundled".into(),
        family: "OpenCanvas Noto Sans".into(),
        weight: 400,
        style: "normal".into(),
        asset_id: None,
        bundled_id: Some("font_bundled".into()),
        content_hash: Some(
            "bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d".into(),
        ),
        license: Some("OFL-1.1; renderer/fonts/OFL.txt".into()),
        status: "ready".into(),
    };
    tx.execute(
        "INSERT INTO fonts(id,data_json) VALUES(?1,?2)",
        params![font.id, serde_json::to_string(&font)?],
    )?;
    tx.commit()?;
    migration::migrate(conn)?;
    Ok(())
}
pub(crate) fn read_meta(conn: &Connection) -> Result<ProjectMeta> {
    Ok(conn.query_row("SELECT project_id,slug,name,schema_version,current_revision FROM project_meta WHERE singleton=1",[],|r| Ok(ProjectMeta { project_id:r.get(0)?,slug:r.get(1)?,name:r.get(2)?,schema_version:r.get(3)?,revision:r.get::<_,i64>(4)? as u64 }))?)
}
fn json_rows<T: serde::de::DeserializeOwned>(conn: &Connection, table: &str) -> Result<Vec<T>> {
    let mut stmt = conn.prepare(&format!("SELECT data_json FROM {table} ORDER BY id"))?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
}
pub(crate) fn read_snapshot(conn: &mut Connection) -> Result<Snapshot> {
    let tx = conn.transaction()?;
    let meta = read_meta(&tx)?;
    let nodes = {
        let mut stmt = tx.prepare("SELECT id,parent_id,sort_key,kind,props_json FROM nodes ORDER BY parent_id,sort_key,id")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;
        rows.map(|r| {
            let (id, parent_id, sort_key, kind, props) = r?;
            Ok(Node {
                id,
                parent_id,
                sort_key,
                kind: serde_json::from_value(Value::String(kind))?,
                props: serde_json::from_str(&props)?,
            })
        })
        .collect::<Result<Vec<_>>>()?
    };
    let tokens = json_rows(&tx, "tokens")?;
    let styles = json_rows(&tx, "styles")?;
    let assets = json_rows(&tx, "assets")?;
    let fonts = json_rows(&tx, "fonts")?;
    let notes = json_rows(&tx, "notes")?;
    let retention = migration::read_retention(&tx)?;
    tx.commit()?;
    Ok(Snapshot {
        schema_version: meta.schema_version,
        project_id: meta.project_id,
        slug: meta.slug,
        name: meta.name,
        revision: meta.revision,
        nodes,
        tokens,
        styles,
        assets,
        fonts,
        notes,
        retention,
    })
}

pub(crate) fn replay(
    conn: &Connection,
    key: &str,
    request_hash: &str,
) -> Result<Option<BatchResult>> {
    let prior: Option<(String, String)> = conn
        .query_row(
            "SELECT request_hash,response_json FROM idempotency WHERE key=?1",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match prior {
        None => Ok(None),
        Some((hash, result)) if hash == request_hash => Ok(Some(serde_json::from_str(&result)?)),
        Some(_) => Err(StorageError::new(
            "IDEMPOTENCY_KEY_REUSED",
            "Idempotency key was already used for a different canonical request",
        )),
    }
}
