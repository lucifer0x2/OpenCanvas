use axum::http::{header, HeaderMap, HeaderValue};

pub(super) fn matches(headers: &HeaderMap, etag: &HeaderValue) -> bool {
    let mut fields = headers.get_all(header::IF_NONE_MATCH).iter();
    let Some(first) = fields.next() else {
        return false;
    };
    if first.as_bytes().trim_ascii() == b"*" {
        return fields.next().is_none();
    }
    std::iter::once(first)
        .chain(fields)
        .try_fold(false, |matched, field| {
            parse_tags(field.as_bytes(), etag.as_bytes()).map(|field| matched || field)
        })
        .unwrap_or(false)
}

fn parse_tags(mut input: &[u8], etag: &[u8]) -> Option<bool> {
    let mut matched = false;
    loop {
        input = input.trim_ascii_start();
        if input.is_empty() {
            return Some(matched);
        }
        if let Some(rest) = input.strip_prefix(b",") {
            input = rest;
            continue;
        }
        let quoted = input.strip_prefix(b"W/").unwrap_or(input);
        let opaque = quoted.strip_prefix(b"\"")?;
        let end = opaque.iter().position(|&byte| byte == b'"')?;
        if opaque[..end]
            .iter()
            .any(|&byte| byte <= b' ' || byte == 127)
        {
            return None;
        }
        matched |= &quoted[..=end + 1] == etag;
        let suffix = opaque[end + 1..].trim_ascii_start();
        if suffix.is_empty() {
            return Some(matched);
        }
        input = suffix.strip_prefix(b",")?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weak_comparison_accepts_lists_and_preserves_quoted_commas() {
        let tag = HeaderValue::from_static("\"known,tag\"");
        let mut headers = HeaderMap::new();
        headers.append(header::IF_NONE_MATCH, HeaderValue::from_static("\"other\""));
        headers.append(
            header::IF_NONE_MATCH,
            HeaderValue::from_static("W/\"known,tag\", ,\"final\""),
        );
        assert!(matches(&headers, &tag));
    }

    #[test]
    fn malformed_lists_and_wildcard_combinations_do_not_match() {
        let tag = HeaderValue::from_static("\"known\"");
        for value in ["*, \"known\"", "\"known\"trailing", "\"known\", \"unclosed"] {
            let mut headers = HeaderMap::new();
            headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static(value));
            assert!(!matches(&headers, &tag));
        }
    }
}
