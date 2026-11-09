mod support;
use opencanvas_text::{Font, FontSet, TextError};
use support::*;

#[test]
fn verifies_original_bytes_and_rejects_malformed_data() {
    assert_eq!(font().hash(), HASH);
    assert_eq!(font().sfnt_bytes(), BYTES);
    let mut changed = BYTES.to_vec();
    changed[0] ^= 1;
    assert!(matches!(
        Font::from_bytes(&changed, HASH),
        Err(TextError::HashMismatch)
    ));
    assert!(matches!(
        Font::from_bytes(BYTES, &HASH.to_uppercase()),
        Err(TextError::InvalidHash)
    ));
    for bytes in [&[][..], b"OTTO", b"wOF2", b"ttcf", b"wOFF"] {
        assert!(matches!(
            Font::from_bytes(bytes, &hash(bytes)),
            Err(TextError::FontDecode)
        ));
    }
}

#[test]
fn structurally_invalid_sfnt_does_not_become_a_ready_font() {
    let mut bytes = BYTES.to_vec();
    bytes[20..24].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(matches!(
        Font::from_bytes(&bytes, &hash(&bytes)),
        Err(TextError::FontDecode)
    ));
    let mut bytes = BYTES.to_vec();
    let tag: [u8; 4] = bytes[12..16].try_into().unwrap();
    bytes[28..32].copy_from_slice(&tag);
    assert!(matches!(
        Font::from_bytes(&bytes, &hash(&bytes)),
        Err(TextError::FontDecode)
    ));
}

#[test]
fn exact_manifest_binding_never_substitutes_another_face() {
    let mut fonts = FontSet::new();
    assert_eq!(fonts.insert("f", font()), Err(TextError::InvalidFace));
    for (family, weight, style) in [
        ("", 400, "normal"),
        ("f", 99, "normal"),
        ("f", 901, "normal"),
        ("f", 400, "oblique"),
    ] {
        assert!(matches!(
            font().with_face(family, weight, style),
            Err(TextError::InvalidFace)
        ));
    }
    let mut fonts = set();
    let regular = font()
        .with_face("OpenCanvas Noto Sans", 400, "normal")
        .unwrap();
    fonts.insert("font_bundled", regular.clone()).unwrap();
    assert_eq!(
        fonts.insert(
            "font_bundled",
            regular.with_face("other", 400, "normal").unwrap()
        ),
        Err(TextError::InvalidFace)
    );
    let base = request("hello");
    for field in 0..5 {
        let mut r = base.clone();
        match field {
            0 => r.font_id = "absent".into(),
            1 => r.font_hash = "0".repeat(64),
            2 => r.family = "wrong".into(),
            3 => r.weight = 700,
            _ => r.style = "italic".into(),
        }
        assert!(matches!(fonts.measure(&r), Err(TextError::FontMissing)));
    }
}

#[test]
fn request_errors_are_explicit_including_empty_text() {
    let fonts = set();
    for field in 0..6 {
        let mut r = request("");
        match field {
            0 => r.font_size = f32::NAN,
            1 => r.line_height = -1.0,
            2 => r.width = Some(f32::INFINITY),
            3 => r.letter_spacing = Some(f32::NAN),
            4 => r.letter_spacing = Some(1001.0),
            _ => r.text_transform = Some("bad".into()),
        }
        assert!(matches!(fonts.measure(&r), Err(TextError::InvalidRequest)));
    }
    let r = request(&"x".repeat(opencanvas_text::MAX_TEXT_BYTES + 1));
    assert!(matches!(fonts.measure(&r), Err(TextError::TextLimit)));
    assert_eq!(TextError::HashMismatch.code(), "RESOURCE_HASH_MISMATCH");
    assert_eq!(TextError::FontDecode.code(), "FONT_MISSING");
}
