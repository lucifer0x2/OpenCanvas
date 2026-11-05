//! Exact numeric comparison, never rounded/toleranced; ink has its own non-gating ledger.
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Comparison {
    pub scenes: usize,
    pub nodes: usize,
    pub texts: usize,
    pub lines: usize,
    pub bounds_fields: usize,
    pub text_fields: usize,
    pub ink_lines: usize,
    pub ink_small: usize,
    pub ink_large: usize,
    pub max_bounds: f64,
    pub max_text: f64,
    pub max_ink: f64,
    pub geometry_errors: Vec<String>,
}

fn index(scene: &Value) -> BTreeMap<&str, &Value> {
    let nodes = scene["nodes"].as_array().unwrap();
    let result: BTreeMap<_, _> = nodes
        .iter()
        .map(|n| (n["id"].as_str().unwrap(), n))
        .collect();
    assert_eq!(result.len(), nodes.len(), "duplicate scene IDs");
    result
}

fn number(a: &Value, b: &Value) -> f64 {
    let (a, b) = (
        a.as_f64().expect("numeric geometry"),
        b.as_f64().expect("numeric geometry"),
    );
    assert!(a.is_finite() && b.is_finite());
    (a - b).abs()
}

impl Comparison {
    pub fn scenes(&mut self, label: &str, browser: &Value, native: &Value, expected: &[String]) {
        let a = index(browser);
        let b = index(native);
        let mut ids = expected.iter().map(String::as_str).collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(
            a.keys().copied().collect::<Vec<_>>(),
            ids,
            "Chromium complete scope {label}"
        );
        assert_eq!(
            b.keys().copied().collect::<Vec<_>>(),
            ids,
            "native complete scope {label}"
        );
        self.scenes += 1;
        let before = (self.nodes, self.texts, self.lines, self.ink_lines);
        for id in ids {
            self.nodes += 1;
            let (left, right) = (a[id], b[id]);
            let link = format!("{label}/node={id}");
            for field in ["kind", "parentId", "effectiveTheme"] {
                if left[field] != right[field] {
                    self.geometry_errors.push(format!(
                        "{link}/{field}: {} != {}",
                        left[field], right[field]
                    ));
                }
            }
            for field in ["x", "y", "width", "height"] {
                self.bounds_fields += 1;
                let difference = number(&left["bounds"][field], &right["bounds"][field]);
                self.max_bounds = self.max_bounds.max(difference);
                if difference != 0.0 {
                    self.geometry_errors.push(format!(
                        "{link}/bounds/{field}: Chromium={} native={} delta={difference}",
                        left["bounds"][field], right["bounds"][field]
                    ));
                }
            }
            let (al, bl) = (
                left["lines"].as_array().unwrap(),
                right["lines"].as_array().unwrap(),
            );
            if al.len() != bl.len() {
                self.geometry_errors.push(format!(
                    "{link}/lineCount: {} != {}",
                    al.len(),
                    bl.len()
                ));
            }
            if left["kind"] != "text" {
                continue;
            }
            self.texts += 1;
            // TextLine has no height member: box height above + declared lineHeight cover it.
            for field in ["fontSize", "lineHeight", "letterSpacing"] {
                self.text_fields += 1;
                let difference = number(&left[field], &right[field]);
                self.max_text = self.max_text.max(difference);
                if difference != 0.0 {
                    self.geometry_errors
                        .push(format!("{link}/{field}: delta={difference}"));
                }
            }
            for (i, (line, other)) in al.iter().zip(bl).enumerate() {
                self.lines += 1;
                let link = format!("{link}/line={i}/text={}", line["text"]);
                self.text_fields += 1;
                if line["text"] != other["text"] {
                    self.geometry_errors
                        .push(format!("{link}: native={}", other["text"]));
                }
                for field in ["x", "y", "width"] {
                    self.text_fields += 1;
                    let difference = number(&line[field], &other[field]);
                    self.max_text = self.max_text.max(difference);
                    if difference != 0.0 {
                        self.geometry_errors.push(format!(
                            "{link}/{field}: Chromium={} native={} delta={difference}",
                            line[field], other[field]
                        ));
                    }
                }
                self.ink(&link, &line["inkBounds"], &other["inkBounds"]);
            }
        }
        eprintln!(
            "SCENE {label} nodes={} texts={} lines={} inkMismatchLines={}",
            self.nodes - before.0,
            self.texts - before.1,
            self.lines - before.2,
            self.ink_lines - before.3
        );
    }

    fn ink(&mut self, link: &str, a: &Value, b: &Value) {
        assert!(
            a.is_object() && b.is_object(),
            "ink must be measured on both sides {link}"
        );
        let mut maximum = 0.0_f64;
        for field in ["x", "y", "width", "height"] {
            let difference = number(&a[field], &b[field]);
            maximum = maximum.max(difference);
            if difference != 0.0 {
                eprintln!(
                    "INK {link}/{field} Chromium={} native={} delta={difference}",
                    a[field], b[field]
                );
            }
        }
        if maximum != 0.0 {
            self.ink_lines += 1;
            self.ink_small += usize::from(maximum <= 1.0);
            self.ink_large += usize::from(maximum > 1.0);
            self.max_ink = self.max_ink.max(maximum);
        }
    }

    pub fn finish(&self) {
        eprintln!("TOTAL scenes={} nodes={} texts={} lines={} boundsFields={} textFields={} geometryErrors={} maxBounds={} maxText={} inkMismatchLines={} inkSmallLE1={} inkLargeGT1={} maxInk={}", self.scenes,self.nodes,self.texts,self.lines,self.bounds_fields,self.text_fields,self.geometry_errors.len(),self.max_bounds,self.max_text,self.ink_lines,self.ink_small,self.ink_large,self.max_ink);
        for error in &self.geometry_errors {
            eprintln!("GEOMETRY {error}");
        }
        assert!(
            self.geometry_errors.is_empty(),
            "{} exact geometry mismatches; see GEOMETRY records",
            self.geometry_errors.len()
        );
    }
}

pub fn no_missing(job: &Value) {
    for issue in job["diagnostics"].as_array().unwrap() {
        assert!(
            !matches!(
                issue["code"].as_str(),
                Some("MISSING_GLYPH" | "GLYPH_CHECK_UNAVAILABLE")
            ),
            "covered fixture cannot be excluded: {issue}"
        );
        if let Some(message) = issue.as_str() {
            assert!(
                !message.starts_with("MISSING_GLYPH")
                    && !message.starts_with("GLYPH_CHECK_UNAVAILABLE"),
                "covered fixture cannot be excluded: {message}"
            );
        }
    }
}
