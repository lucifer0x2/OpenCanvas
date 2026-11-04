//! Shared authored syntax for responsive layout.

pub const MAX_BREAKPOINTS: usize = 8;

/// Parses a literal decimal percentage as a unit fraction.
pub fn percentage(value: &str) -> Option<f32> {
    let digits = value.strip_suffix('%')?;
    let (whole, fraction) = match digits.split_once('.') {
        Some((whole, fraction)) if !fraction.is_empty() => (whole, fraction),
        Some(_) => return None,
        None => (digits, ""),
    };
    if whole.is_empty()
        || !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() {
        0
    } else {
        whole.parse::<u8>().ok()?
    };
    if whole > 100 || (whole == 100 && fraction.bytes().any(|b| b != b'0')) {
        return None;
    }
    Some(digits.parse::<f32>().ok()? / 100.0)
}

/// Widths must be in range and strictly increasing from the initial sentinel zero.
pub fn valid_breakpoint_width(width: u32, previous: u32) -> bool {
    width > previous && width <= 100_000
}
