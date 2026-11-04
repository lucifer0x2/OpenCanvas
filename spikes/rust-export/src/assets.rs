//! Files the CLI reads that the library takes as bytes: the repo font.
use crate::font::Shaper;

pub const FONT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../renderer/fonts/NotoSans.ttf");

pub fn shaper() -> Shaper {
    Shaper::from_bytes(std::fs::read(FONT_PATH).expect("font"))
}
