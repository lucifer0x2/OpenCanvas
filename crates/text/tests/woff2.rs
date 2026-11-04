mod support;
use opencanvas_text::{Font, TextError};
use std::io::Cursor;
use support::*;

fn base128(n: usize, output: &mut Vec<u8>) {
    let mut shift = 28;
    while shift > 0 && n >> shift == 0 {
        shift -= 7;
    }
    loop {
        let byte = ((n >> shift) & 127) as u8;
        output.push(if shift > 0 { byte | 128 } else { byte });
        if shift == 0 {
            break;
        }
        shift -= 7;
    }
}

/// Encode the existing OFL fixture without WOFF2 transforms. No downloaded fixture or
/// system font; Brotli encoding is a TEST-only dependency, never in the runtime library.
fn fixture() -> Vec<u8> {
    let count = u16::from_be_bytes([BYTES[4], BYTES[5]]) as usize;
    let mut directory = Vec::new();
    let mut tables = Vec::new();
    for record in BYTES[12..12 + count * 16].as_chunks::<16>().0 {
        let tag = &record[..4];
        // For glyf/loca, transform version 3 denotes the null transform.
        directory.push(if tag == b"glyf" || tag == b"loca" {
            255
        } else {
            63
        });
        directory.extend_from_slice(tag);
        let at = u32::from_be_bytes(record[8..12].try_into().unwrap()) as usize;
        let len = u32::from_be_bytes(record[12..16].try_into().unwrap()) as usize;
        base128(len, &mut directory);
        tables.extend_from_slice(&BYTES[at..at + len]);
    }
    let mut compressed = Vec::new();
    let params = brotli::enc::BrotliEncoderParams {
        quality: 1,
        ..Default::default()
    };
    brotli::BrotliCompress(&mut Cursor::new(tables), &mut compressed, &params).unwrap();
    let length = 48 + directory.len() + compressed.len();
    let mut out = vec![0u8; 48];
    out[..4].copy_from_slice(b"wOF2");
    out[4..8].copy_from_slice(&BYTES[..4]);
    out[8..12].copy_from_slice(&(length as u32).to_be_bytes());
    out[12..14].copy_from_slice(&(count as u16).to_be_bytes());
    out[16..20].copy_from_slice(&(BYTES.len() as u32).to_be_bytes());
    out[20..24].copy_from_slice(&(compressed.len() as u32).to_be_bytes());
    out.extend(directory);
    out.extend(compressed);
    out
}

#[test]
fn woff2_keeps_original_hash_and_exposes_reconstructed_sfnt() {
    let woff = fixture();
    let original_hash = hash(&woff);
    let decoded = Font::from_bytes(&woff, &original_hash).unwrap();
    assert_eq!(decoded.hash(), original_hash);
    assert_ne!(decoded.hash(), hash(decoded.sfnt_bytes()));
    assert_eq!(&decoded.sfnt_bytes()[..4], &BYTES[..4]);
    for text in ["AVATAR", "Tiếng Việt ă â đ ê ô ơ ư", "ffi"] {
        assert_eq!(
            decoded.positioned_run(text, 16.0, 0.0).unwrap().advance,
            font().positioned_run(text, 16.0, 0.0).unwrap().advance
        );
    }
    assert!(matches!(
        Font::from_bytes(&woff, HASH),
        Err(TextError::HashMismatch)
    ));
}

#[test]
fn malformed_and_oversized_woff2_fail_without_unbounded_output() {
    let good = fixture();
    for len in [0, 3, 47, 48, good.len() - 1] {
        let bad = &good[..len];
        assert!(Font::from_bytes(bad, &hash(bad)).is_err());
    }
    let mut bad = vec![0u8; 48];
    bad[..4].copy_from_slice(b"wOF2");
    bad[4..8].copy_from_slice(&BYTES[..4]);
    bad[8..12].copy_from_slice(&54u32.to_be_bytes());
    bad[12..14].copy_from_slice(&1u16.to_be_bytes());
    bad[16..20].copy_from_slice(&u32::MAX.to_be_bytes());
    bad.push(63);
    bad.extend_from_slice(b"name");
    base128(u32::MAX as usize, &mut bad);
    let length = bad.len() as u32;
    bad[8..12].copy_from_slice(&length.to_be_bytes());
    assert!(matches!(
        Font::from_bytes(&bad, &hash(&bad)),
        Err(TextError::FontDecode)
    ));
}
