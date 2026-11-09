//! LTR logical script items, independent of grapheme-safe wrapping boundaries.
//! This is not paragraph bidi, language detection or Unicode Script_Extensions.
use harfrust::{unicode, Script};
use std::ops::Range;
use unicode_bidi::{BidiDataSource, HardcodedBidiData};

pub(crate) struct Item {
    pub range: Range<usize>,
    pub script: Script,
}

fn strong(script: Script) -> bool {
    !matches!(script, Script::COMMON | Script::INHERITED | Script::UNKNOWN)
}

pub(crate) fn items(text: &str) -> Vec<Item> {
    let mut result: Vec<Item> = Vec::new();
    // Inherited marks join their base, but a strong-script mark can begin a new
    // shaping item inside an extended grapheme. Wrapping still uses whole graphemes.
    let mut start = 0;
    let mut current = Script::COMMON;
    let mut brackets = Vec::new();
    for (at, c) in text.char_indices() {
        let property = unicode::script(c as u32);
        let mut script = strong(property).then_some(property);
        if let Some(pair) = HardcodedBidiData.bidi_matched_opening_bracket(c) {
            if pair.is_open {
                brackets.push((pair.opening, current));
            } else if let Some(index) = brackets.iter().rposition(|(open, _)| *open == pair.opening)
            {
                script = Some(brackets[index].1);
                brackets.truncate(index);
            }
        }
        let Some(script) = script else { continue };
        if !strong(current) {
            current = script;
            for (_, bracket_script) in &mut brackets {
                if !strong(*bracket_script) {
                    *bracket_script = script;
                }
            }
        } else if script != current {
            result.push(Item {
                range: start..at,
                script: current,
            });
            start = at;
            current = script;
        }
    }
    if !text.is_empty() {
        result.push(Item {
            range: start..text.len(),
            script: current,
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(text: &str) -> Vec<(&str, Script)> {
        items(text)
            .into_iter()
            .map(|i| (&text[i.range], i.script))
            .collect()
    }

    #[test]
    fn mixed_scripts_keep_inherited_but_split_strong_script_marks() {
        assert_eq!(
            split("ς i\u{307}"),
            vec![("ς ", Script::GREEK), ("i\u{307}", Script::LATIN)]
        );
        assert_eq!(
            split(" . i\u{307} ς\u{301}!"),
            vec![
                (" . i\u{307} ", Script::LATIN),
                ("ς\u{301}!", Script::GREEK)
            ]
        );
        assert_eq!(
            split("\u{301} (Ж) a\u{483} α"),
            vec![
                ("\u{301} (Ж) ", Script::CYRILLIC),
                ("a", Script::LATIN),
                ("\u{483} ", Script::CYRILLIC),
                ("α", Script::GREEK)
            ]
        );
        assert_eq!(
            split(" \u{301}.\u{200b}"),
            vec![(" \u{301}.\u{200b}", Script::COMMON)]
        );
        assert!(items("").is_empty());
    }

    #[test]
    fn unicode_classifier_is_not_a_latin_greek_special_case() {
        assert_eq!(
            split("aЖअก漢"),
            vec![
                ("a", Script::LATIN),
                ("Ж", Script::CYRILLIC),
                ("अ", Script::DEVANAGARI),
                ("ก", Script::THAI),
                ("漢", Script::HAN)
            ]
        );
    }

    #[test]
    fn paired_common_brackets_recover_the_openers_script() {
        assert_eq!(
            split("α[(i\u{307})]ς"),
            vec![
                ("α[(", Script::GREEK),
                ("i\u{307}", Script::LATIN),
                (")]ς", Script::GREEK)
            ]
        );
        assert_eq!(
            split("（i\u{307}α）b"),
            vec![
                ("（i\u{307}", Script::LATIN),
                ("α", Script::GREEK),
                ("）b", Script::LATIN)
            ]
        );
    }
}
