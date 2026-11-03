use super::*;
use crate::{Font, ProjectMeta, StorageError, SCHEMA_VERSION};
use ::postgres::{GenericClient, Row};

pub(super) struct ProjectRow {
    pub meta: ProjectMeta,
    pub namespace: String,
    pub generation: i64,
    pub oldest: u64,
}

pub(super) fn decode(row: Row) -> Result<ProjectRow> {
    let name: Vec<u8> = row.get("name");
    Ok(ProjectRow {
        meta: ProjectMeta {
            project_id: row.get("project_id"),
            slug: row.get("slug"),
            name: String::from_utf8(name)
                .map_err(|_| StorageError::new("PROJECT_INVALID", "Project manifest is invalid"))?,
            schema_version: row.get::<_, i32>("schema_version") as u32,
            revision: row.get::<_, i64>("revision") as u64,
        },
        generation: row.get("generation"),
        namespace: row.get("namespace"),
        oldest: row.get::<_, i64>("oldest_revision") as u64,
    })
}

// Slugs can resemble IDs. Prefer the canonical ID if an ambiguous key exists.
macro_rules! lookup_sql {
    ($suffix:literal) => {
        concat!("SELECT projects.*,(SELECT namespace FROM schema_registry WHERE singleton) AS namespace FROM projects WHERE project_id=$1 OR slug=$1 ORDER BY (project_id=$1) DESC LIMIT 1", $suffix)
    };
}
const LOOKUP: &str = lookup_sql!("");
const LOOKUP_FOR_UPDATE: &str = lookup_sql!(" FOR UPDATE");
pub(super) const LIST: &str =
    "SELECT projects.*,(SELECT namespace FROM schema_registry WHERE singleton) AS namespace FROM projects";

pub(super) fn lookup<C: GenericClient>(
    db: &mut Db<'_, C>,
    key: &str,
    lock: bool,
) -> Result<ProjectRow> {
    if !crate::validate::valid_id(key) {
        return Err(StorageError::validation(
            "projectId",
            "Invalid project identifier",
        ));
    }
    let query = if lock { LOOKUP_FOR_UPDATE } else { LOOKUP };
    let row = db
        .query_opt(query, &[&key])?
        .ok_or_else(|| StorageError::new("PROJECT_NOT_FOUND", "Project does not exist"))?;
    let project = decode(row)?;
    if project.meta.schema_version != SCHEMA_VERSION {
        return Err(StorageError::new(
            "SCHEMA_UNSUPPORTED",
            "Project schema requires a supported migration before writing",
        ));
    }
    Ok(project)
}

pub(super) fn create(
    client: &mut ::postgres::Client,
    slug: &str,
    name: &str,
) -> Result<ProjectMeta> {
    crate::validate_slug(slug)?;
    if name.is_empty() || name.len() > 256 {
        return Err(StorageError::validation(
            "name",
            "Project name must contain 1–256 bytes",
        ));
    }
    let meta = ProjectMeta {
        project_id: format!("p_{}", uuid::Uuid::new_v4().simple()),
        slug: slug.into(),
        name: name.into(),
        schema_version: SCHEMA_VERSION,
        revision: 0,
    };
    let mut tx = client.transaction().map_err(sql)?;
    tx.execute(
        "INSERT INTO projects(project_id,slug,name,schema_version,revision) VALUES($1,$2,$3,$4,0)",
        &[
            &meta.project_id,
            &slug,
            &name.as_bytes(),
            &(SCHEMA_VERSION as i32),
        ],
    )
    .map_err(|error| {
        if error.code() == Some(&::postgres::error::SqlState::UNIQUE_VIOLATION) {
            StorageError::new("PROJECT_EXISTS", "A project with this slug already exists")
        } else {
            sql(error)
        }
    })?;
    let font = Font {
        id: "font_bundled".into(),
        family: "OpenCanvas Noto Sans".into(),
        weight: 400,
        style: "normal".into(),
        asset_id: None,
        bundled_id: Some("font_bundled".into()),
        content_hash: Some(crate::hash(crate::bundle::BUNDLED_FONT)),
        license: Some("OFL-1.1; renderer/fonts/OFL.txt".into()),
        status: "ready".into(),
    };
    tx.execute(
        "INSERT INTO entities(project_id,kind,id,data) VALUES($1,'font',$2,$3)",
        &[&meta.project_id, &font.id, &serde_json::to_string(&font)?],
    )
    .map_err(sql)?;
    tx.commit().map_err(sql)?;
    Ok(meta)
}
