//! ICC serialization hygiene only: deterministic date and four-byte tag alignment.
use moxcms::ColorProfile;

pub fn encode(profile: &ColorProfile) -> Vec<u8> {
    let bytes = profile.encode().unwrap();
    let count = u32::from_be_bytes(bytes[128..132].try_into().unwrap()) as usize;
    assert!(count <= 128 && bytes.len() < 4 * 1024 * 1024);
    let mut out = bytes[..132 + count * 12].to_vec();
    // Fixed fixture creation date, no wall clock or machine identity in originals.
    out[24..36].copy_from_slice(&[7, 234, 0, 11, 0, 4, 0, 0, 0, 0, 0, 0]);
    for i in 0..count {
        let tag = 132 + i * 12;
        let offset = u32::from_be_bytes(bytes[tag + 4..tag + 8].try_into().unwrap()) as usize;
        let size = u32::from_be_bytes(bytes[tag + 8..tag + 12].try_into().unwrap()) as usize;
        assert!(offset + size <= bytes.len());
        out.resize(out.len().next_multiple_of(4), 0);
        let new_offset = out.len() as u32;
        out[tag + 4..tag + 8].copy_from_slice(&new_offset.to_be_bytes());
        out.extend_from_slice(&bytes[offset..offset + size]);
    }
    out.resize(out.len().next_multiple_of(4), 0);
    let size = out.len() as u32;
    out[..4].copy_from_slice(&size.to_be_bytes());
    ColorProfile::new_from_slice(&out).unwrap();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_tags_are_aligned_bounded_and_deterministic() {
        let p = ColorProfile::new_display_p3();
        let bytes = encode(&p);
        assert_eq!(bytes, encode(&p));
        assert_eq!(bytes.len() % 4, 0);
        let count = u32::from_be_bytes(bytes[128..132].try_into().unwrap()) as usize;
        for tag in bytes[132..132 + count * 12].as_chunks::<12>().0 {
            let offset = u32::from_be_bytes(tag[4..8].try_into().unwrap()) as usize;
            let size = u32::from_be_bytes(tag[8..12].try_into().unwrap()) as usize;
            assert_eq!(offset % 4, 0);
            assert!(offset + size <= bytes.len());
        }
    }
}
