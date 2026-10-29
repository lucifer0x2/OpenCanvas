//! The CSS easing grammar `props.animations` accepts (`animation-timing-function`).
use crate::error::*;
use serde_json::Value;

const KEYWORDS: [&str; 5] = ["linear", "ease", "ease-in", "ease-out", "ease-in-out"];
const STEP_POSITIONS: [&str; 4] = ["start", "end", "jump-start", "jump-end"];
const MESSAGE: &str = "Use linear, ease, ease-in, ease-out, ease-in-out, \
cubic-bezier(x1,y1,x2,y2) with x1,x2 in 0..1, or steps(n[, start|end])";

pub(super) fn check_easing(value: &Value, field: &str) -> Result<()> {
    if value.as_str().is_some_and(valid) {
        Ok(())
    } else {
        Err(StorageError::validation(field, MESSAGE))
    }
}

fn valid(easing: &str) -> bool {
    let easing = easing.trim();
    if KEYWORDS.contains(&easing) {
        return true;
    }
    if let Some(args) = arguments(easing, "cubic-bezier") {
        let numbers: Option<Vec<f64>> = args.iter().map(|a| a.parse().ok()).collect();
        return numbers.is_some_and(|n| {
            n.len() == 4
                && n.iter().all(|v| v.is_finite())
                && (0. ..=1.).contains(&n[0])
                && (0. ..=1.).contains(&n[2])
        });
    }
    if let Some(args) = arguments(easing, "steps") {
        let count = args.first().and_then(|n| n.parse::<u32>().ok());
        return count.is_some_and(|n| (1..=1000).contains(&n))
            && match args.len() {
                1 => true,
                2 => STEP_POSITIONS.contains(&args[1]),
                _ => false,
            };
    }
    false
}

/// The comma-separated, trimmed arguments of `name(...)`.
fn arguments<'a>(easing: &'a str, name: &str) -> Option<Vec<&'a str>> {
    let inner = easing.strip_prefix(name)?.trim_start();
    let inner = inner.strip_prefix('(')?.strip_suffix(')')?;
    Some(inner.split(',').map(str::trim).collect())
}

#[cfg(test)]
mod tests {
    use super::valid;

    #[test]
    fn accepts_the_css_grammar_and_nothing_else() {
        for ok in [
            "ease",
            "ease-in-out",
            "cubic-bezier(0.34,1.56,0.64,1)",
            "cubic-bezier( 0, -2, 1, 3 )",
            "steps(4, end)",
            "steps(1)",
            "steps(3,jump-start)",
        ] {
            assert!(valid(ok), "{ok}");
        }
        for bad in [
            "cubic-bezier(1.2,0,0,1)",
            "cubic-bezier(0,0,1)",
            "steps(0)",
            "steps(2.5)",
            "steps(2, middle)",
            "spring(1)",
            "easeOut",
            "",
        ] {
            assert!(!valid(bad), "{bad}");
        }
    }
}
