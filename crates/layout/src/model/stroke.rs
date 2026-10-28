//! Box stroke: authored input (one value or 4 sides) and the normalized scene form.
use crate::util::{err, finite};
use serde::{Deserialize, Serialize};

/// One value for every side, or exactly 4 in CSS order [top, right, bottom, left].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Sides<T> {
    All(T),
    Each([T; 4]),
}

impl<T: Clone> Sides<T> {
    pub fn each(&self) -> [T; 4] {
        match self {
            Self::All(v) => [v.clone(), v.clone(), v.clone(), v.clone()],
            Self::Each(v) => v.clone(),
        }
    }
}

/// Resolved authored `stroke`: literal colors and widths, defaults not yet applied.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrokeInput {
    pub color: Sides<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<Sides<f32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
}

/// Scene stroke, normalized to 4 sides [top, right, bottom, left].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneStroke {
    pub color: [String; 4],
    pub width: [f32; 4],
    pub align: String,
}

impl StrokeInput {
    pub fn widths(&self) -> [f32; 4] {
        self.width.as_ref().map_or([1.0; 4], Sides::each)
    }
    pub fn align(&self) -> &str {
        self.align.as_deref().unwrap_or("inside")
    }
    pub(crate) fn check(&self) -> Result<(), super::LayoutError> {
        if !self.widths().into_iter().all(finite)
            || !matches!(self.align(), "inside" | "center" | "outside")
        {
            return Err(err("Invalid stroke"));
        }
        Ok(())
    }
    /// The scene form, or `None` when every side is 0 so stroke-less scenes stay unchanged.
    pub fn scene(&self) -> Option<SceneStroke> {
        let width = self.widths();
        width.iter().any(|w| *w > 0.0).then(|| SceneStroke {
            color: self.color.each(),
            width,
            align: self.align().into(),
        })
    }
    /// Space an inside stroke takes from the content box, like a CSS border [t, r, b, l].
    pub fn inside_insets(&self) -> [f32; 4] {
        if self.align() == "inside" {
            self.widths()
        } else {
            [0.0; 4]
        }
    }
}
