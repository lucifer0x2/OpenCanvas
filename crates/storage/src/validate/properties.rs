use super::{values::*, Candidate};
use crate::{error::*, model::*};
use serde_json::Value;
impl Candidate {
    pub(super) fn properties(&self, n: &Node) -> Result<()> {
        let props = self.resolve_props(n)?;
        let p = props.as_object().unwrap();
        let allowed = [
            "name",
            "x",
            "y",
            "width",
            "height",
            "minWidth",
            "maxWidth",
            "minHeight",
            "maxHeight",
            "layout",
            "text",
            "fontId",
            "fontSize",
            "lineHeight",
            "fill",
            "assetId",
            "fit",
            "opacity",
            "clip",
            "radius",
            "crop",
        ];
        for (k, v) in p {
            if !allowed.contains(&k.as_str()) {
                return Err(StorageError::validation(
                    "props",
                    "Unsupported authored property",
                ));
            }
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
                "width" | "height" => {
                    if matches!(v.as_str(), Some("fill" | "hug")) {
                    } else {
                        number(v, 0., 100_000., k)?;
                    }
                    if v == "fill" {
                        if let Some(parent) = n.parent_id.as_ref().and_then(|id| self.nodes.get(id))
                        {
                            let parent_props = self.resolve_props(parent)?;
                            if parent_props.get(k).is_none()
                                || parent_props.get(k) == Some(&Value::String("hug".into()))
                            {
                                return Err(StorageError::validation(
                                    k,
                                    "Fill inside a hug axis is ambiguous and unsupported",
                                ));
                            }
                        }
                    }
                }
                "minWidth" | "maxWidth" | "minHeight" | "maxHeight" | "radius" => {
                    number(v, 0., 100_000., k)?
                }
                "fontSize" | "lineHeight" => number(v, 1., 10000., k)?,
                "opacity" => number(v, 0., 1., k)?,
                "fill" => color(v)?,
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
                "fit" => {
                    if !matches!(v.as_str(), Some("fit" | "contain" | "cover")) {
                        return Err(StorageError::validation(
                            k,
                            "Only fit, contain and cover are supported",
                        ));
                    }
                }
                "text" => {
                    if v.as_str().is_none_or(|s| s.len() > 64 * 1024) {
                        return Err(StorageError::validation(
                            k,
                            "Text must be a plain string up to 64 KiB",
                        ));
                    }
                }
                "layout" => {
                    if !n.kind.container() {
                        return Err(StorageError::validation(
                            k,
                            "Only containers support layout",
                        ));
                    }
                    let l = v
                        .as_object()
                        .ok_or_else(|| StorageError::validation(k, "Layout must be an object"))?;
                    if !matches!(
                        l.get("mode").and_then(Value::as_str),
                        Some("row" | "column" | "free")
                    ) {
                        return Err(StorageError::validation(
                            "layout.mode",
                            "Only row, column and free layout are supported",
                        ));
                    }
                    for (k, v) in l {
                        match k.as_str() {
                            "mode" => {}
                            "padding" | "gap" => number(v, 0., 100_000., k)?,
                            _ => {
                                return Err(StorageError::validation(
                                    "layout",
                                    "Unsupported layout property",
                                ))
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        for axis in ["Width", "Height"] {
            if let (Some(min), Some(max)) = (
                p.get(&format!("min{axis}")).and_then(Value::as_f64),
                p.get(&format!("max{axis}")).and_then(Value::as_f64),
            ) {
                if min > max {
                    return Err(StorageError::validation(
                        "bounds",
                        "Minimum must not exceed maximum",
                    ));
                }
            }
        }
        if n.kind == NodeKind::Text {
            for key in ["text", "fontId", "fontSize", "lineHeight"] {
                if !p.contains_key(key) {
                    return Err(StorageError::validation(key,"Text requires explicit text, fontId, fontSize and lineHeight (or a text style)"));
                }
            }
        } else if p
            .keys()
            .any(|k| ["text", "fontId", "fontSize", "lineHeight"].contains(&k.as_str()))
        {
            return Err(StorageError::validation(
                "props",
                "Text properties require a text node",
            ));
        }
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
        } else if p.contains_key("assetId") || p.contains_key("fit") || p.contains_key("crop") {
            return Err(StorageError::validation(
                "props",
                "Image properties require an image node",
            ));
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
