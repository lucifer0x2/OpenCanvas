use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_BATCH_OPS: usize = 500;
pub const MAX_BATCH_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_ASSET_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_NODES: usize = 20_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Page,
    Frame,
    Group,
    Rectangle,
    Text,
    Image,
}
impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Frame => "frame",
            Self::Group => "group",
            Self::Rectangle => "rectangle",
            Self::Text => "text",
            Self::Image => "image",
        }
    }
    pub fn container(&self) -> bool {
        matches!(self, Self::Page | Self::Frame | Self::Group)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub parent_id: Option<String>,
    pub sort_key: i64,
    pub kind: NodeKind,
    pub props: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Token {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub token_type: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Style {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub content_hash: String,
    pub media_type: String,
    pub byte_size: u64,
    /// Opaque, content addressed key relative to assets/. Never an absolute path.
    pub relative_key: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Font {
    pub id: String,
    pub family: String,
    pub weight: u16,
    pub style: String,
    pub asset_id: Option<String>,
    pub bundled_id: Option<String>,
    pub content_hash: Option<String>,
    pub license: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMeta {
    pub project_id: String,
    pub slug: String,
    pub name: String,
    pub schema_version: u32,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Retention {
    pub oldest_commit_revision: u64,
    pub idempotency_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub schema_version: u32,
    pub project_id: String,
    pub slug: String,
    pub name: String,
    pub revision: u64,
    pub nodes: Vec<Node>,
    pub tokens: Vec<Token>,
    pub styles: Vec<Style>,
    pub assets: Vec<Asset>,
    pub fonts: Vec<Font>,
    pub retention: Retention,
}

mod requests;
pub use requests::*;
