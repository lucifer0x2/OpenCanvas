use rmcp::{model::*, ErrorData};

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

pub(crate) fn list() -> ListResourcesResult {
    ListResourcesResult {
        resources: SKILLS
            .iter()
            .map(|(uri, name, text)| {
                Resource::new(*uri, *name)
                    .with_mime_type("text/markdown")
                    .with_size(text.len() as u64)
            })
            .collect(),
        ..Default::default()
    }
}

pub(crate) fn read(uri: &str) -> Result<ReadResourceResponse, ErrorData> {
    let (_, _, text) = SKILLS
        .iter()
        .find(|(known, _, _)| *known == uri)
        .ok_or_else(|| ErrorData::resource_not_found("Unknown OpenCanvas skill resource", None))?;
    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(*text, uri).with_mime_type("text/markdown")
    ])
    .into())
}
