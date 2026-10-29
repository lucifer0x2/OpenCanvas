//! Which painter `Painter::render` runs, chosen with `--paint full|bbox`.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaintMode {
    /// Original Part D painter: whole-canvas text masks, a mask copy per parent (gate reference).
    Full,
    /// Text masks over glyph bounds, inherited clips borrowed, parallel passes (default).
    Bbox,
}

impl PaintMode {
    pub fn parse(args: &[String]) -> PaintMode {
        match args.iter().position(|a| a == "--paint").map(|i| args[i + 1].as_str()) {
            None | Some("bbox") => PaintMode::Bbox,
            Some("full") => PaintMode::Full,
            Some(other) => panic!("--paint full|bbox, not {other}"),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PaintMode::Full => "full",
            PaintMode::Bbox => "bbox",
        }
    }
}
