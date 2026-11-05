//! Original bounded fixtures; no external profiles, OS services or alternative CMS.
mod cmyk;
mod profile;
use image::{codecs::png::PngEncoder, ExtendedColorType, ImageDecoder, ImageEncoder};
use moxcms::ColorProfile;
use serde_json::{json, Value};
use std::{env, error::Error, fs, io::Cursor, path::Path};

fn png(pixels: &[u8], color: ExtendedColorType, icc: Option<&[u8]>) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = PngEncoder::new(&mut bytes);
    if let Some(profile) = icc {
        encoder.set_icc_profile(profile.to_vec()).unwrap();
    }
    encoder.write_image(pixels, 32, 16, color).unwrap();
    bytes
}

fn pattern(gray: bool, alpha: bool) -> Vec<u8> {
    (0..16)
        .flat_map(|_| {
            (0..32).flat_map(move |x| {
                let i = x / 8;
                let a = if alpha { [128, 64, 192, 0][i] } else { 255 };
                if gray {
                    vec![[32, 96, 128, 192][i], a]
                } else {
                    [[180, 100, 100], [80, 170, 110], [100, 120, 190], [128; 3]][i]
                        .into_iter()
                        .chain([a])
                        .collect()
                }
            })
        })
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let arg = env::args().nth(1).ok_or("usage: icc-fixtures OUT_ROOT")?;
    let out = Path::new(&arg);
    fs::create_dir_all(out.join("resources"))?;
    fs::create_dir_all(out.join("profiles"))?;
    let profiles = [
        ("display-p3", ColorProfile::new_display_p3(), false),
        ("srgb", ColorProfile::new_srgb(), false),
        ("gray-linear", ColorProfile::new_gray_with_gamma(1.0), true),
    ];
    let mut fixtures = Vec::<Value>::new();
    for (name, profile, gray) in profiles {
        let icc = profile::encode(&profile);
        let parsed = ColorProfile::new_from_slice(&icc)?;
        if parsed.color_space != profile.color_space {
            return Err("ICC roundtrip color space".into());
        }
        fs::write(out.join("profiles").join(format!("{name}.icc")), &icc)?;
        for alpha in [false, true] {
            let suffix = if alpha { "alpha" } else { "opaque" };
            let base = format!("{name}-{suffix}");
            let pixels = pattern(gray, alpha);
            let color = if gray {
                ExtendedColorType::La8
            } else {
                ExtendedColorType::Rgba8
            };
            for tagged in [true, false] {
                let id = format!("{base}-{}", if tagged { "icc" } else { "untagged" });
                let bytes = png(&pixels, color, tagged.then_some(icc.as_slice()));
                let mut decoder = image::codecs::png::PngDecoder::new(Cursor::new(&bytes))?;
                if decoder.icc_profile()?.as_deref() != tagged.then_some(icc.as_slice()) {
                    return Err("embedded profile identity".into());
                }
                fs::write(out.join("resources").join(format!("{id}.png")), bytes)?;
                fixtures.push(
                    json!({"id": id, "file": format!("{id}.png"), "mediaType": "image/png",
                    "width": 32, "height": 16, "profile": name, "tagged": tagged, "alpha": alpha,
                    "pair": base, "iccParse": if tagged { "valid" } else { "absent" }}),
                );
            }
        }
    }
    let cmyk = profile::encode(&cmyk::profile());
    ColorProfile::new_from_slice(&cmyk)?;
    fs::write(out.join("profiles/cmyk-constant-xyz.icc"), &cmyk)?;
    for tagged in [true, false] {
        let id = format!("cmyk-{}", if tagged { "icc" } else { "untagged" });
        fs::write(
            out.join("resources").join(format!("{id}.jpg")),
            cmyk::jpeg(tagged.then_some(&cmyk)),
        )?;
        fixtures.push(
            json!({"id": id, "file": format!("{id}.jpg"), "mediaType": "image/jpeg",
            "width": 8, "height": 8, "profile": "cmyk-constant-xyz", "tagged": tagged,
            "alpha": false, "pair": "cmyk", "iccParse": if tagged { "valid" } else { "absent" }}),
        );
    }
    // Deliberately invalid ICC is ONLY an error-policy probe, never a success fixture.
    let invalid = b"not-an-icc-profile";
    fs::write(
        out.join("resources/malformed-icc.png"),
        png(
            &pattern(false, false),
            ExtendedColorType::Rgba8,
            Some(invalid),
        ),
    )?;
    fs::write(
        out.join("fixtures.json"),
        serde_json::to_vec_pretty(&fixtures)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_gray_profiles_roundtrip_and_corrupt_profile_rejects() {
        for p in [
            ColorProfile::new_display_p3(),
            ColorProfile::new_srgb(),
            ColorProfile::new_gray_with_gamma(1.0),
        ] {
            let bytes = p.encode().unwrap();
            assert_eq!(
                ColorProfile::new_from_slice(&bytes).unwrap().color_space,
                p.color_space
            );
            assert!(ColorProfile::new_from_slice(&bytes[..64]).is_err());
        }
        assert!(ColorProfile::new_from_slice(b"not-an-icc-profile").is_err());
    }

    #[test]
    fn gray_alpha_and_rgb_payloads_preserve_original_samples() {
        for gray in [false, true] {
            let pixels = pattern(gray, true);
            let color = if gray {
                ExtendedColorType::La8
            } else {
                ExtendedColorType::Rgba8
            };
            let bytes = png(&pixels, color, None);
            let image = image::load_from_memory(&bytes).unwrap();
            assert_eq!(
                if gray {
                    image.into_luma_alpha8().into_raw()
                } else {
                    image.into_rgba8().into_raw()
                },
                pixels
            );
        }
    }
}
