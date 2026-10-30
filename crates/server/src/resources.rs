use crate::{cache_hints::Cached, props_doc};
use rmcp::{model::*, ErrorData};
#[path = "capabilities.rs"]
mod capabilities;

const SKILLS: [(&str, &str, &str); 2] = [
    (
        "skill://opencanvas/implement-frame.md",
        "Implement an OpenCanvas frame",
        include_str!("../skills/implement-frame.md"),
    ),
    (
        "skill://opencanvas/author-frame.md",
        "Author an OpenCanvas frame",
        include_str!("../skills/author-frame.md"),
    ),
];

/// Static skills plus the property reference generated from the validator's table.
fn skills() -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
    SKILLS
        .into_iter()
        .chain([(props_doc::URI, props_doc::NAME, props_doc::text())])
}

pub(crate) fn list() -> ListResourcesResult {
    ListResourcesResult::with_all_items(
        skills()
            .map(|(uri, name, text)| {
                Resource::new(uri, name)
                    .with_mime_type("text/markdown")
                    .with_size(text.len() as u64)
            })
            .chain([Resource::new(capabilities::URI, capabilities::NAME)
                .with_mime_type("application/json")
                .with_size(capabilities::text().len() as u64)])
            .collect(),
    )
    .cached()
}

/// No templates are advertised; the empty list still carries the cache hints that
/// protocol 2026-07-28 requires (rmcp's default handler omits them).
pub(crate) fn templates() -> ListResourceTemplatesResult {
    ListResourceTemplatesResult::default().cached()
}

pub(crate) fn read(uri: &str) -> Result<ReadResourceResponse, ErrorData> {
    if uri == capabilities::URI {
        return Ok(ReadResourceResult::new(vec![ResourceContents::text(
            capabilities::text(),
            uri,
        )
        .with_mime_type("application/json")])
        .cached()
        .into());
    }
    let (_, _, text) = skills()
        .find(|(known, _, _)| *known == uri)
        .ok_or_else(|| ErrorData::resource_not_found("Unknown OpenCanvas skill resource", None))?;
    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(text, uri).with_mime_type("text/markdown")
    ])
    .cached()
    .into())
}
