use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Sizing {
    Fixed(f32),
    Rule(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutRule {
    #[serde(default = "free")]
    pub mode: String,
    #[serde(default)]
    pub padding: f32,
    #[serde(default)]
    pub gap: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub wrap: bool,
}
fn free() -> String {
    "free".into()
}
fn is_false(value: &bool) -> bool {
    !value
}
