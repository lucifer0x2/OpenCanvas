use super::*;
use crate::ResourceBytes;
use opencanvas_layout::{SceneDecoration, TextLine};
use sha2::{Digest, Sha256};

const FONT: &[u8] = include_bytes!("../../../../renderer/fonts/NotoSans.ttf");
fn text_scene(text: &str) -> (PaintScene, String) {
    let hash = format!("{:x}", Sha256::digest(FONT));
    let mut n = node("text", 3.0, 2.0, 56.0, 54.0, "#123456");
    n.kind = "text".into();
    n.font_id = Some("font_bundled".into());
    n.lines = vec![TextLine {
        text: text.into(),
        x: 0.0,
        y: 24.0,
        width: 50.0,
        ink_bounds: None,
    }];
    let mut s = scene(vec![n]);
    s.fonts.push(serde_json::from_value(json!({"id":"font_bundled","family":"Fixture","hash":hash,"weight":400,"style":"normal","url":"unused"})).unwrap());
    (s, hash)
}
fn resources(hash: &str) -> Resources<'_> {
    let mut r = Resources::default();
    r.fonts
        .insert("font_bundled".into(), ResourceBytes { hash, bytes: FONT });
    r
}
#[test]
fn paints_only_given_lines_baselines_and_tracking_with_shared_run() {
    let (mut s, hash) = text_scene("Việt");
    let r = crate::render(&s, &resources(&hash), &RenderOptions::default()).unwrap();
    assert_eq!(r.draw.drawn_lines, 1);
    assert!(r.rgba.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
    assert_eq!(pixel(&r, 10, 50), [0; 4]);
    // Scene width never triggers a second wrap or clipping of ink.
    s.nodes[0].bounds.width = 1.0;
    let same = crate::render(&s, &resources(&hash), &RenderOptions::default()).unwrap();
    assert_eq!(same.rgba, r.rgba);
    s.nodes[0].letter_spacing = 3.0;
    let tracked = crate::render(&s, &resources(&hash), &RenderOptions::default()).unwrap();
    assert_ne!(tracked.rgba, r.rgba);
}
#[test]
fn missing_glyph_is_advisory_in_strict_mode_and_notdef_is_drawn() {
    let (s, hash) = text_scene("\u{10ffff}");
    let r = crate::render(&s, &resources(&hash), &RenderOptions::default()).unwrap();
    assert!(r
        .diagnostics
        .iter()
        .any(|d| d["code"] == "MISSING_GLYPH" && d["codepoints"][0] == 0x10ffff));
    assert!(r.rgba.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
}
#[test]
fn decoration_overlap_uses_one_opacity_layer_and_experimental_masks_are_explicit() {
    let (mut s, hash) = text_scene("HƯƠNG");
    let n = &mut s.nodes[0];
    n.opacity = 0.5;
    n.decoration = Some(SceneDecoration {
        line: "line-through".into(),
        offset: -6.0,
        thickness: 2.0,
    });
    let r = crate::render(&s, &resources(&hash), &RenderOptions::default()).unwrap();
    assert_eq!(r.draw.opacity_layers, 1);
    assert!(r.rgba.as_chunks::<4>().0.iter().all(|p| p[3] <= 128));
    let o = RenderOptions {
        text_coverage_gamma: 1.45,
        snap_glyph_origins: true,
        ..RenderOptions::default()
    };
    let variant = crate::render(&s, &resources(&hash), &o).unwrap();
    assert_ne!(variant.rgba, r.rgba);
}
#[test]
fn font_hash_identity_and_decode_failure_are_not_system_fallback() {
    let (s, hash) = text_scene("test");
    let mut r = Resources::default();
    r.fonts.insert(
        "font_bundled".into(),
        ResourceBytes {
            hash: &hash,
            bytes: b"bad",
        },
    );
    assert_eq!(
        crate::render(&s, &r, &RenderOptions::default())
            .unwrap_err()
            .code,
        "PIPELINE_MISMATCH"
    );
    let mut s = s;
    let hash = format!("{:x}", Sha256::digest(b"bad"));
    s.fonts[0].hash = hash.clone();
    let mut r = Resources::default();
    r.fonts.insert(
        "font_bundled".into(),
        ResourceBytes {
            hash: &hash,
            bytes: b"bad",
        },
    );
    assert_eq!(
        crate::render(&s, &r, &RenderOptions::default())
            .unwrap_err()
            .code,
        "RESOURCE_NOT_READY"
    );
}
#[test]
fn color_font_tables_fail_explicitly_instead_of_monochrome_output() {
    let mut bytes = FONT.to_vec();
    let count = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    let entry = bytes[12..12 + count * 16]
        .as_chunks_mut::<16>()
        .0
        .iter_mut()
        .find(|entry| &entry[..4] == b"name")
        .unwrap();
    // A directory-preserving synthetic unsupported table (no licensed external font).
    entry[..4].copy_from_slice(b"COLR");
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let (mut s, _) = text_scene("test");
    s.fonts[0].hash = hash.clone();
    let mut r = Resources::default();
    r.fonts.insert(
        "font_bundled".into(),
        ResourceBytes {
            hash: &hash,
            bytes: &bytes,
        },
    );
    assert_eq!(
        crate::render(&s, &r, &RenderOptions::default())
            .unwrap_err()
            .code,
        "UNSUPPORTED_FEATURE"
    );
}
