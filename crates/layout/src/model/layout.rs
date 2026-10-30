use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Sizing {
    Fixed(f32),
    Rule(String),
}

/// Stack padding: one number, or 1–4 numbers with CSS shorthand meaning.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Padding {
    All(f32),
    Sides(Vec<f32>),
}

impl Default for Padding {
    fn default() -> Self {
        Self::All(0.0)
    }
}

impl Padding {
    /// `[top, right, bottom, left]`, or `None` for an empty or over-long list.
    pub fn edges(&self) -> Option<[f32; 4]> {
        match *self {
            Self::All(v) => Some([v; 4]),
            Self::Sides(ref s) => super::stroke::expand_quad(s),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutRule {
    #[serde(default = "free")]
    pub mode: String,
    #[serde(default)]
    pub padding: Padding,
    #[serde(default)]
    pub gap: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub wrap: bool,
    /// Cross-axis alignment: start (default), center, end or stretch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    /// Main-axis distribution: start (default), center, end or space-between.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justify: Option<String>,
}
fn free() -> String {
    "free".into()
}
fn is_false(value: &bool) -> bool {
    !value
}
