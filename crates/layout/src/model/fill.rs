//! Literal paint boundary. Solid scenes retain their historical string representation.
use crate::util::err;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Paint {
    Solid(String),
    Gradient(Gradient),
}
impl From<&str> for Paint {
    fn from(v: &str) -> Self {
        Self::Solid(v.into())
    }
}
impl From<String> for Paint {
    fn from(v: String) -> Self {
        Self::Solid(v)
    }
}
impl Paint {
    pub fn as_solid(&self) -> &str {
        match self {
            Self::Solid(s) => s,
            _ => "transparent",
        }
    }
    pub(crate) fn check(&self, text: bool) -> Result<(), super::LayoutError> {
        let valid = match self {
            Self::Solid(s) => solid(s) || s == "transparent",
            Self::Gradient(g) => !text && g.valid(),
        };
        if valid {
            Ok(())
        } else {
            Err(err(
                "Invalid fill; gradients require a box and 2..16 ordered color stops",
            ))
        }
    }
}
pub(super) fn solid(s: &str) -> bool {
    s.strip_prefix('#').is_some_and(|h| {
        [3, 4, 6, 8].contains(&h.len()) && h.bytes().all(|b| b.is_ascii_hexdigit())
    })
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Gradient {
    Linear {
        #[serde(default = "angle")]
        angle: f32,
        stops: Vec<(f32, String)>,
    },
    Radial {
        #[serde(default = "center")]
        center: [f32; 2],
        #[serde(default)]
        radius: Option<Radius>,
        stops: Vec<(f32, String)>,
    },
}
fn angle() -> f32 {
    180.0
}
fn center() -> [f32; 2] {
    [0.5, 0.5]
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Radius {
    Circle(f32),
    Ellipse([f32; 2]),
}
impl Gradient {
    fn valid(&self) -> bool {
        let stops = match self {
            Self::Linear { angle, stops } => {
                if !angle.is_finite() || angle.abs() > 360.0 {
                    return false;
                }
                stops
            }
            Self::Radial {
                center,
                radius,
                stops,
            } => {
                if center.iter().any(|v| !v.is_finite() || v.abs() > 10.0) {
                    return false;
                }
                let radii = match radius {
                    None => [1.0; 2],
                    Some(Radius::Circle(r)) => [*r; 2],
                    Some(Radius::Ellipse(r)) => *r,
                };
                if radii
                    .iter()
                    .any(|r| !r.is_finite() || *r <= 0.0 || *r > 100_000.0)
                {
                    return false;
                }
                stops
            }
        };
        (2..=16).contains(&stops.len())
            && stops
                .iter()
                .all(|(p, c)| p.is_finite() && (0.0..=1.0).contains(p) && solid(c))
            && stops.windows(2).all(|w| w[0].0 <= w[1].0)
    }
}
