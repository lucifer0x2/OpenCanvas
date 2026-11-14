//! On `use`, a key that is not an instance prop names a master descendant (`ph="x"`,
//! `ph.r=8`). A mistyped key would otherwise become an override target, so it is checked
//! here: a node prop that instances do not take fails, and when the master is a `comp` of
//! this source, a target that is not one of its descendants fails with the closest
//! descendant ID or instance key as the suggestion. Storage checks stored masters the same
//! way (`props.overrides.<target>`).
use super::error::Result;
use super::keys::effective_key;
use super::parser::Part;
use opencanvas_storage::{instances::INSTANCE_KEYS, suggest_among};

/// Checks the descendant targets of a `use` node's parts. `descendants` lists the master's
/// descendant IDs when the master is created earlier in the same source.
pub fn check(parts: &[Part], descendants: Option<&[String]>) -> Result<()> {
    for part in parts {
        let Part::Attr(key, _) = part else { continue };
        let (target, dotted) = match key.text.split_once('.') {
            Some((target, _)) => (target, true),
            None => (key.text.as_str(), false),
        };
        let known = descendants.is_some_and(|ids| ids.iter().any(|id| id == target));
        if known || (!dotted && (key.text == "props" || instance_key(target))) {
            continue;
        }
        let at = key.slice(0, Some(target.chars().count()));
        if !dotted && effective_key(target).is_some() {
            return Err(at.error(format!(
                "`{target}` is not an instance property; override a master descendant with <descendantId>.{target}=…"
            )));
        }
        if let Some(ids) = descendants {
            let candidates = ids.iter().map(String::as_str);
            let near = suggest_among(target, candidates.chain(INSTANCE_KEYS.iter().copied()));
            return Err(at
                .error(format!(
                    "`{target}` is neither an instance property nor a descendant of the master component"
                ))
                .suggest(near));
        }
    }
    Ok(())
}

fn instance_key(key: &str) -> bool {
    effective_key(key).is_some_and(|key| INSTANCE_KEYS.contains(&key))
}
