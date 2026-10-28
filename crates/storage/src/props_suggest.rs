//! "Did you mean" lookup for an unsupported authored property key.
use crate::props_catalog::PROPS;

/// The catalog key an agent most likely meant: a known alias, a case-insensitive match,
/// then the closest key within two edits. `None` when nothing is plausibly close.
pub fn suggest_prop(key: &str) -> Option<&'static str> {
    let lower = key.to_ascii_lowercase();
    if let Some(spec) = PROPS.iter().find(|spec| {
        spec.key.eq_ignore_ascii_case(key)
            || spec
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(key))
    }) {
        return Some(spec.key);
    }
    // Short keys tolerate one edit so "id" does not turn into "x".
    let tolerance = (lower.len() / 3).clamp(1, 2);
    PROPS
        .iter()
        .map(|spec| {
            (
                edit_distance(&lower, &spec.key.to_ascii_lowercase()),
                spec.key,
            )
        })
        .filter(|(distance, _)| *distance <= tolerance)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, key)| key)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitute = previous + usize::from(ca != *cb);
            previous = row[j + 1];
            row[j + 1] = substitute.min(row[j] + 1).min(previous + 1);
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_case_and_typos_map_to_catalog_keys() {
        assert_eq!(suggest_prop("cornerRadius"), Some("radius"));
        assert_eq!(suggest_prop("color"), Some("fill"));
        assert_eq!(suggest_prop("FONTSIZE"), Some("fontSize"));
        assert_eq!(suggest_prop("opacty"), Some("opacity"));
        assert_eq!(suggest_prop("padding"), Some("layout"));
        assert_eq!(suggest_prop("letterSpacing"), None);
        assert_eq!(suggest_prop("id"), None);
    }

    #[test]
    fn aliases_never_shadow_a_real_key() {
        for spec in PROPS {
            for alias in spec.aliases {
                assert!(
                    PROPS
                        .iter()
                        .all(|other| !other.key.eq_ignore_ascii_case(alias)),
                    "{alias} is both an alias and a key"
                );
            }
        }
    }
}
