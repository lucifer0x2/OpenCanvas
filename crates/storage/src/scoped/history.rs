//! Versioned private history; public readers continue returning operation arrays.
use super::index::PageIndex;
use crate::*;

pub(crate) fn request_value(request: &BatchRequest, pages: Option<&[String]>) -> Result<Value> {
    let mut value = serde_json::to_value(request)?;
    if let Some(pages) = pages {
        value["pageIds"] = json!(pages);
    }
    Ok(value)
}

pub(crate) fn operations(value: Value) -> Value {
    if value["scopeVersion"] == 1 {
        value["operations"].clone()
    } else {
        value
    }
}

pub(crate) fn footprint(before: &Snapshot, after: &Candidate) -> Result<BTreeSet<String>> {
    footprint_indexed(
        before,
        after,
        None,
        &PageIndex::new(&before.nodes),
        &PageIndex::new(after.nodes.values()),
    )
}

pub(super) fn footprint_indexed(
    before: &Snapshot,
    after: &Candidate,
    changed: Option<&[String]>,
    old_pages: &PageIndex,
    new_pages: &PageIndex,
) -> Result<BTreeSet<String>> {
    let old_nodes: BTreeMap<_, _> = before.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let ids: BTreeSet<_> = match changed {
        Some(ids) => ids.iter().map(String::as_str).collect(),
        None => old_nodes
            .keys()
            .copied()
            .chain(after.nodes.keys().map(String::as_str))
            .collect(),
    };
    let mut keys = BTreeSet::new();
    bench_phase::count("history.compared_ids", ids.len());
    if super::navigation::changed(&old_nodes, after, &ids, old_pages, new_pages) {
        keys.insert("*".into());
    }
    for id in &ids {
        if !same(old_nodes.get(id).copied(), after.nodes.get(*id))? {
            keys.insert(format!("node:{id}"));
            for pages in [old_pages, new_pages] {
                if let Some(page) = pages.page(id) {
                    keys.insert(format!("page:{page}"));
                }
            }
        }
    }
    macro_rules! diff {
        ($field:ident, $kind:literal) => {
            let old: BTreeMap<_, _> = before.$field.iter().map(|v| (v.id.as_str(), v)).collect();
            let resource_ids: BTreeSet<_> = match changed {
                Some(_) => ids.clone(),
                None => old
                    .keys()
                    .copied()
                    .chain(after.$field.keys().map(String::as_str))
                    .collect(),
            };
            for id in resource_ids {
                if !same(old.get(id).copied(), after.$field.get(id))? {
                    keys.insert(format!(concat!($kind, ":{}"), id));
                }
            }
        };
    }
    diff!(tokens, "token");
    diff!(styles, "style");
    diff!(assets, "asset");
    diff!(fonts, "font");
    diff!(notes, "note");
    let old_fonts: BTreeMap<_, _> = before.fonts.iter().map(|f| (f.id.as_str(), f)).collect();
    for font in before.fonts.iter().chain(after.fonts.values()) {
        if changed.is_some() && !ids.contains(font.id.as_str()) {
            continue;
        }
        if !same(
            old_fonts.get(font.id.as_str()).copied(),
            after.fonts.get(&font.id),
        )? {
            keys.insert(format!("font-family:{}", font.family));
        }
    }
    // An empty footprint is safe and explicitly represented, unlike absent coverage.
    Ok(keys)
}

fn same<T: serde::Serialize>(before: Option<&T>, after: Option<&T>) -> Result<bool> {
    Ok(serde_json::to_vec(&before)? == serde_json::to_vec(&after)?)
}

pub(crate) fn envelope(
    before: &Snapshot,
    after: &Candidate,
    request: Value,
    operations: Value,
) -> Result<Value> {
    envelope_with_scopes(footprint(before, after)?, request, operations)
}

pub(crate) fn envelope_with_scopes(
    scopes: BTreeSet<String>,
    request: Value,
    operations: Value,
) -> Result<Value> {
    let mut value = json!({"scopeVersion":1,"request":request,"operations":operations,
        "scopes":scopes});
    seal(&mut value)?;
    Ok(value)
}

fn seal(value: &mut Value) -> Result<()> {
    value.as_object_mut().unwrap().remove("scopeHash");
    value["scopeHash"] = json!(hash(&serde_json::to_vec(&canonical_json(value))?));
    Ok(())
}

pub(crate) fn coverage(value: &Value, project: &str, base: u64) -> Result<Vec<String>> {
    let mut sealed = value.clone();
    seal(&mut sealed)?;
    if sealed != *value {
        return Err(StorageError::new(
            "HISTORY_INVALID",
            "Scoped history checksum is invalid",
        ));
    }
    let key = value["request"]["idempotencyKey"]
        .as_str()
        .unwrap_or("resource");
    if value["operations"]
        .as_array()
        .is_none_or(|ops| ops.is_empty() || ops.len() > MAX_BATCH_OPS)
    {
        return Err(StorageError::new(
            "HISTORY_INVALID",
            "Invalid history operations",
        ));
    }
    crate::bundle_history::replay_request(project, base as i64, key, value.clone())?;
    let scopes: Vec<String> = serde_json::from_value(value["scopes"].clone())?;
    for scope in &scopes {
        if scope == "*" {
            continue;
        }
        let valid = scope.split_once(':').is_some_and(|(kind, id)| match kind {
            "page" | "node" | "token" | "style" | "asset" | "font" | "note" => {
                validate::valid_id(id)
            }
            "font-family" => !id.is_empty() && id.len() <= 256,
            _ => false,
        });
        if !valid {
            return Err(StorageError::new("HISTORY_INVALID", "Invalid scope key"));
        }
    }
    Ok(scopes)
}

/// Restored history preserves replay bodies, but cannot grant trust to imported
/// scope metadata. Any guard crossing these commits must refetch.
pub(crate) fn scrub(conn: &Connection) -> Result<()> {
    let values = {
        let mut statement = conn.prepare("SELECT revision,ops_json FROM commits")?;
        let rows =
            statement.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()?
    };
    for (revision, text) in values {
        let mut value: Value = serde_json::from_str(&text)?;
        if value["scopeVersion"] == 1 {
            value["scopes"] = json!(["*"]);
            seal(&mut value)?;
            conn.execute(
                "UPDATE commits SET ops_json=?1 WHERE revision=?2",
                params![serde_json::to_string(&value)?, revision],
            )?;
        }
    }
    Ok(())
}
