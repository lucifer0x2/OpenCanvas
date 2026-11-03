//! Two actual compiled client contracts for a narrow additive migration fixture.
use crate::{blobs, db};
use oc_spike_core::{DomainError, State};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Create {
    project_id: String,
    #[cfg(feature = "compat-v2")]
    #[serde(default)]
    description: String,
}

pub async fn create(url: &str, bytes: &[u8]) -> Result<Value, DomainError> {
    let request: Create = serde_json::from_slice(bytes).map_err(blobs::invalid)?;
    if request.project_id.is_empty()
        || request.project_id.len() > 128
        || !request
            .project_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
    {
        return Err(blobs::invalid("Invalid project ID"));
    }
    let client = db::connect(url).await?;
    #[cfg(feature = "compat-v2")]
    let changed = {
        if request.description.len() > 4096 {
            return Err(blobs::invalid("Description exceeds 4096 bytes"));
        }
        client
            .execute(
                "INSERT INTO projects(project_id,description) VALUES($1,$2) ON CONFLICT DO NOTHING",
                &[&request.project_id, &request.description],
            )
            .await
            .map_err(db::internal)?
    };
    #[cfg(not(feature = "compat-v2"))]
    let changed = client
        .execute(
            "INSERT INTO projects(project_id) VALUES($1) ON CONFLICT DO NOTHING",
            &[&request.project_id],
        )
        .await
        .map_err(db::internal)?;
    if changed == 0 {
        return Err(DomainError::new("PROJECT_EXISTS", "Project already exists"));
    }
    let value = serde_json::to_value(State {
        project_id: request.project_id,
        revision: 0,
        nodes: vec![],
    })
    .map_err(db::internal)?;
    #[cfg(feature = "compat-v2")]
    let value = {
        let mut value = value;
        value["description"] = Value::String(request.description);
        value
    };
    Ok(value)
}

pub async fn snapshot(url: &str, id: &str) -> Result<Value, DomainError> {
    Ok(crate::hot_snapshot::load(url, id).await?.1)
}
