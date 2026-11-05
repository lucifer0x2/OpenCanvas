/// Selects the compute and PNG pipeline. Chromium remains the default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RenderBackend {
    #[default]
    Chromium,
    Rust,
}

impl std::str::FromStr for RenderBackend {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "chromium" => Ok(Self::Chromium),
            "rust" => Ok(Self::Rust),
            _ => Err("Render backend must be chromium or rust"),
        }
    }
}
