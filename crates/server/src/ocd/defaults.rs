//! Size defaults written for a text node. They never defeat the parent's layout: a cross-axis
//! size under `align=stretch` would stop the stretch, `fill` under `center`/`end` would pin
//! the text to the start, and under `@in` the existing parent's layout is unknown. A parent
//! with `layout.breakpoints` has one layout per rule; an axis gets a default only when every
//! rule gives the same one.
use serde_json::{Map, Value};

type Size = (Option<&'static str>, Option<&'static str>);

/// What a new child needs to know about the container written above it in the source.
pub struct Parent {
    /// The base layout, then each breakpoint's; `None` when a breakpoint is malformed (the
    /// batch then fails in storage with the real error, so no default is guessed).
    rules: Option<Vec<Rule>>,
    /// Authored width other than `hug`: storage rejects `fill` under a hug or unset width.
    /// Breakpoints replace the layout only, so this is the same for every rule.
    definite_width: bool,
}

/// The parts of one layout rule that decide a text child's defaults.
struct Rule {
    /// `row`, `column`, `free`, or `None` without a layout.
    mode: Option<String>,
    align: Option<String>,
}

impl Parent {
    pub fn of(props: &Map<String, Value>) -> Self {
        Self {
            rules: rules(props.get("layout")),
            definite_width: props.get("width").is_some_and(|w| w != "hug"),
        }
    }
}

fn rules(layout: Option<&Value>) -> Option<Vec<Rule>> {
    let mut rules = vec![Rule::of(layout)];
    let Some(entries) = layout.and_then(|l| l.get("breakpoints")) else {
        return Some(rules);
    };
    for entry in entries.as_array()? {
        let rule = entry.get("layout").filter(|l| l.is_object())?;
        rules.push(Rule::of(Some(rule)));
    }
    Some(rules)
}

impl Rule {
    fn of(layout: Option<&Value>) -> Self {
        let field = |key: &str| {
            let value = layout.and_then(|l| l.get(key));
            value.and_then(Value::as_str).map(String::from)
        };
        Self {
            mode: field("mode"),
            align: field("align"),
        }
    }

    fn text_size(&self, definite_width: bool) -> Size {
        let align = self.align.as_deref().unwrap_or("start");
        match self.mode.as_deref() {
            Some("column") => {
                let width = match align {
                    "stretch" => None,
                    "start" if definite_width => Some("fill"),
                    _ => Some("hug"),
                };
                (width, Some("hug"))
            }
            Some("row") => (Some("hug"), (align != "stretch").then_some("hug")),
            _ => (Some("hug"), Some("hug")),
        }
    }
}

/// `(width, height)` for a text child; `None` writes nothing (an unset text size is its
/// content size, and the size a stretching stack fills). `parent` is `None` under `@in`.
pub fn text_size(parent: Option<&Parent>) -> Size {
    let Some((parent, rules)) = parent.and_then(|p| Some((p, p.rules.as_deref()?))) else {
        return (None, None);
    };
    let sizes: Vec<Size> = rules
        .iter()
        .map(|rule| rule.text_size(parent.definite_width))
        .collect();
    let agreed = |axis: fn(&Size) -> Option<&'static str>| {
        let first = axis(&sizes[0]);
        let same = sizes.iter().all(|size| axis(size) == first);
        same.then_some(first).flatten()
    };
    (agreed(|s| s.0), agreed(|s| s.1))
}
