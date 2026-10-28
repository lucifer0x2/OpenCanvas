use super::{
    links::check_links,
    prop_keys::{check_keys, check_required},
    stroke::validate_stroke,
    values::*,
    Candidate,
};
use crate::{error::*, model::*};
use serde_json::Value;
impl Candidate {
    pub(super) fn properties(&self, n: &Node) -> Result<()> {
        let props = self.resolve_props(n)?;
        let p = props.as_object().unwrap();
        check_keys(&n.kind, p)?;
        self.layout_properties(n, p)?;
        for (k, v) in p {
            match k.as_str() {
                "name" => {
                    if v.as_str().is_none_or(|s| s.len() > 256) {
                        return Err(StorageError::validation(
                            "name",
                            "Name must be a string up to 256 bytes",
                        ));
                    }
                }
                "x" | "y" => number(v, -100_000., 100_000., k)?,
                "radius" => number(v, 0., 100_000., k)?,
                "fontSize" | "lineHeight" => number(v, 1., 10000., k)?,
                "opacity" => number(v, 0., 1., k)?,
                "fill" => color(v)?,
                "exportSettings" => crate::check_export_settings(v, "exportSettings")?,
                "stroke" => validate_stroke(v)?,
                "links" => check_links(v)?,
                "flowStart" => self.check_flow_start(n, v)?,
                "clip" => {
                    if !v.is_boolean() {
                        return Err(StorageError::validation(k, "Clip must be a boolean"));
                    }
                }
                "crop" => {
                    let crop = v.as_object().ok_or_else(|| {
                        StorageError::validation("crop", "Crop must be an object")
                    })?;
                    if crop.len() != 4 {
                        return Err(StorageError::validation(
                            "crop",
                            "Crop requires normalized x, y, width and height",
                        ));
                    }
                    for key in ["x", "y", "width", "height"] {
                        number(
                            crop.get(key).unwrap_or(&Value::Null),
                            if key == "width" || key == "height" {
                                0.000001
                            } else {
                                0.
                            },
                            1.,
                            "crop",
                        )?;
                    }
                    if crop["x"].as_f64().unwrap() + crop["width"].as_f64().unwrap() > 1.
                        || crop["y"].as_f64().unwrap() + crop["height"].as_f64().unwrap() > 1.
                    {
                        return Err(StorageError::validation(
                            "crop",
                            "Normalized crop must stay inside the source image",
                        ));
                    }
                }
                "fontId" => {
                    if !self.fonts.contains_key(v.as_str().unwrap_or("")) {
                        return Err(StorageError::validation(
                            k,
                            "Referenced font does not exist",
                        ));
                    }
                }
                "assetId" => {
                    if !self.assets.contains_key(v.as_str().unwrap_or("")) {
                        return Err(StorageError::validation(
                            k,
                            "Referenced asset does not exist",
                        ));
                    }
                }
                "fit" => crate::image_fit::check_fit(v)?,
                "text" if v.as_str().is_none_or(|s| s.len() > 64 * 1024) => {
                    return Err(StorageError::validation(
                        k,
                        "Text must be a plain string up to 64 KiB",
                    ));
                }
                _ => {}
            }
        }
        check_required(&n.kind, p)?;
        if n.kind == NodeKind::Image {
            let asset = p
                .get("assetId")
                .and_then(Value::as_str)
                .and_then(|a| self.assets.get(a))
                .ok_or_else(|| {
                    StorageError::validation("assetId", "Image requires an existing asset")
                })?;
            if !asset.media_type.starts_with("image/") {
                return Err(StorageError::validation(
                    "assetId",
                    "Image requires an image asset",
                ));
            }
        }
        if n.kind == NodeKind::Frame
            && n.parent_id
                .as_ref()
                .and_then(|p| self.nodes.get(p))
                .is_some_and(|p| p.kind == NodeKind::Page)
        {
            for key in ["width", "height"] {
                if !p.get(key).is_some_and(Value::is_number) {
                    return Err(StorageError::validation(
                        key,
                        "Root frames require fixed width and height",
                    ));
                }
            }
        }
        Ok(())
    }
}
