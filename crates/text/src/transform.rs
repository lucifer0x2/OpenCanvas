use crate::TextError;
use unicode_segmentation::UnicodeSegmentation;

/// Locale-independent CSS casing. Rust's string lowercasing handles contextual final
/// sigma; titlecase changes only the first letter of each Unicode word, not its tail.
pub fn transform_text(text: &str, transform: Option<&str>) -> Result<String, TextError> {
    match transform.unwrap_or("none") {
        "none" => Ok(text.to_owned()),
        "uppercase" => Ok(text.to_uppercase()),
        "lowercase" => Ok(text.to_lowercase()),
        "capitalize" => {
            let mut result = String::with_capacity(text.len());
            let mut end = 0;
            for (start, word) in text.unicode_word_indices() {
                result.push_str(&text[end..start]);
                let mut found = false;
                for c in word.chars() {
                    if !found && c.is_alphanumeric() {
                        found = true;
                        let title = unicode_case_mapping::to_titlecase(c);
                        // Blink's capitalize uses ICU's simple (single-scalar)
                        // titlecase, not full mappings such as ß -> Ss or ﬃ -> Ffi.
                        if title[0] == 0 || title[1] != 0 {
                            result.push(c);
                        } else {
                            result.extend(
                                title
                                    .into_iter()
                                    .filter_map(char::from_u32)
                                    .filter(|c| *c != '\0'),
                            );
                        }
                    } else {
                        result.push(c);
                    }
                }
                end = start + word.len();
            }
            result.push_str(&text[end..]);
            Ok(result)
        }
        _ => Err(TextError::InvalidRequest),
    }
}
