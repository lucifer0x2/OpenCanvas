//! Shadow defaults normalize only at the scene boundary, keeping authored props intact.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shadow {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub blur: f32,
    #[serde(default)]
    pub spread: f32,
    pub color: String,
    #[serde(default)]
    pub inset: bool,
}
impl Shadow {
    pub(crate) fn valid(&self) -> bool {
        [
            (self.x, -10000., 10000.),
            (self.y, -10000., 10000.),
            (self.blur, 0., 1000.),
            (self.spread, -1000., 1000.),
        ]
        .into_iter()
        .all(|(v, min, max)| v.is_finite() && (min..=max).contains(&v))
            && super::fill::solid(&self.color)
    }
}
