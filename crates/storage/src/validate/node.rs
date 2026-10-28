use super::{values::*, Candidate};
use crate::{error::*, model::*};
use std::collections::BTreeSet;
impl Candidate {
    pub(super) fn style(&self, style: &Style) -> Result<()> {
        id(&style.id)?;
        if style.name.len() > 256 {
            return Err(StorageError::validation("name", "Name exceeds 256 bytes"));
        }
        match style.kind.as_str() {
            "fill" => {
                if style.value.get("styleId").is_some() {
                    return Err(StorageError::validation(
                        "style.value",
                        "Style aliases are not supported",
                    ));
                }
                self.resolve_value(&style.value, "color")?;
            }
            "text" => {
                let o = style.value.as_object().ok_or_else(|| {
                    StorageError::validation("value", "Text style must be a property object")
                })?;
                for (k, v) in o {
                    match k.as_str() {
                        "fontId" => {
                            if !self.fonts.contains_key(v.as_str().unwrap_or("")) {
                                return Err(StorageError::validation(
                                    "fontId",
                                    "Referenced font does not exist",
                                ));
                            }
                        }
                        "fill" => {
                            if v.get("styleId").is_some() {
                                return Err(StorageError::validation(
                                    "style.value",
                                    "Style aliases are not supported",
                                ));
                            }
                            self.resolve_value(v, "color")?;
                        }
                        "fontSize" | "lineHeight" => {
                            number(&self.resolve_value(v, "dimension")?, 1., 10000., k)?;
                        }
                        _ => {
                            return Err(StorageError::validation(
                                "style.value",
                                "Unsupported text style property",
                            ))
                        }
                    }
                }
            }
            _ => {
                return Err(StorageError::validation(
                    "style.kind",
                    "Only primitive fill and text styles are supported",
                ))
            }
        }
        Ok(())
    }
    pub(super) fn node(&self, n: &Node) -> Result<()> {
        id(&n.id)?;
        match (&n.kind, &n.parent_id) {
            (NodeKind::Page, None) => {}
            (NodeKind::Page, Some(_)) => {
                return Err(StorageError::validation("parentId", "Pages must be roots"))
            }
            (_, None) => {
                return Err(StorageError::validation(
                    "parentId",
                    "Non-page nodes require a parent",
                ))
            }
            (_, Some(p)) => {
                let parent = self.nodes.get(p).ok_or_else(|| {
                    StorageError::validation(
                        "parentId",
                        "Parent does not exist; forward references are not supported",
                    )
                })?;
                if !parent.kind.container() {
                    return Err(StorageError::validation(
                        "parentId",
                        "Parent must be a page, frame or group",
                    ));
                }
                let mut ancestors = BTreeSet::new();
                let mut cursor = Some(n.id.as_str());
                while let Some(k) = cursor {
                    if !ancestors.insert(k) {
                        return Err(StorageError::validation("parentId", "Tree cycle detected"));
                    }
                    if ancestors.len() > 128 {
                        return Err(StorageError::validation(
                            "parentId",
                            "Tree exceeds maximum depth 128",
                        ));
                    }
                    cursor = self.nodes.get(k).and_then(|n| n.parent_id.as_deref());
                }
            }
        }
        self.properties(n)?;
        Ok(())
    }
}
