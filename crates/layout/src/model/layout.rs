use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Sizing {
    Fixed(f32),
    Rule(String),
}

impl Sizing {
    pub(crate) fn percentage(&self) -> Option<f32> {
        match self {
            Self::Rule(rule) => crate::percentage(rule),
            Self::Fixed(_) => None,
        }
    }
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layout {
    #[serde(flatten, deserialize_with = "base_rule")]
    pub base: LayoutRule,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub breakpoints: Vec<LayoutBreakpoint>,
}

fn base_rule<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<LayoutRule, D::Error> {
    let mut props = serde_json::Map::deserialize(deserializer)?;
    props.entry("mode").or_insert("free".into());
    serde_json::from_value(props.into()).map_err(serde::de::Error::custom)
}

impl Layout {
    pub(crate) fn rules(&self) -> impl Iterator<Item = &LayoutRule> {
        std::iter::once(&self.base).chain(self.breakpoints.iter().map(|entry| &entry.layout))
    }

    pub(crate) fn at_width(&self, width: f64) -> &LayoutRule {
        self.breakpoints
            .iter()
            .find(|entry| width <= f64::from(entry.max_width))
            .map_or(&self.base, |entry| &entry.layout)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutBreakpoint {
    pub max_width: u32,
    pub layout: LayoutRule,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutRule {
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
fn is_false(value: &bool) -> bool {
    !value
}
