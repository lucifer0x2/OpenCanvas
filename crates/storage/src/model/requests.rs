use super::*;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    CreateNode {
        id: String,
        kind: NodeKind,
        #[serde(rename = "parentId")]
        parent_id: Option<String>,
        props: Value,
        #[serde(default)]
        index: Option<usize>,
    },
    UpdateNode {
        id: String,
        props: Value,
    },
    ReparentNode {
        id: String,
        #[serde(rename = "parentId")]
        parent_id: String,
        #[serde(default)]
        index: Option<usize>,
    },
    ReorderNode {
        id: String,
        index: usize,
    },
    DeleteSubtree {
        id: String,
    },
    UpsertToken {
        token: Token,
    },
    UpsertStyle {
        style: Style,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchRequest {
    pub project_id: String,
    pub expected_revision: u64,
    pub idempotency_key: String,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    pub project_id: String,
    pub committed_revision: u64,
    pub changed_ids: Vec<String>,
    pub render_status: String,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub revision: u64,
    pub base_revision: u64,
    pub batch_id: String,
    pub operations: Value,
    pub changed_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImportAssetRequest {
    pub project_id: String,
    pub expected_revision: u64,
    pub idempotency_key: String,
    pub media_type: String,
    pub bytes: Vec<u8>,
    /// Optional font registration, atomically committed with asset metadata.
    pub font: Option<FontRegistration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontRegistration {
    pub id: String,
    pub family: String,
    pub weight: u16,
    pub style: String,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAssetResult {
    pub asset: Asset,
    pub result: BatchResult,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchMetrics {
    pub queue_wait_us: u64,
    pub read_us: u64,
    pub validate_us: u64,
    pub sql_write_us: u64,
    pub commit_us: u64,
    pub total_us: u64,
    pub request_bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkpoint {
    pub busy: i64,
    pub wal_pages: i64,
    pub checkpointed_pages: i64,
}

/// BTreeMap ordering supplies canonical object-key ordering recursively.
pub fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), canonical_json(v)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}
