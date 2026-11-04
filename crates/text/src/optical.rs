//! CSS instance selection with one final 2.14 rounding, without OS/name rules.
use harfrust::{
    font::{variation::Axis, NormalizedCoord},
    Font, Tag,
};
use read_fonts::{types::MajorMinor, TableProvider};

pub(crate) fn at_size(font: &Font, size: f32, location: &[(Tag, f32)]) -> Font {
    if font.axes().get_by_tag(Tag::new(b"opsz")).is_none() {
        return font.clone();
    }
    if font
        .tables()
        .avar()
        .ok()
        .is_none_or(|a| a.version() == MajorMinor::VERSION_1_0)
    {
        return at_location(font, &[(Tag::new(b"opsz"), size)]);
    }
    // Complete user location is necessary for avar2's cross-axis adjustment.
    let mut settings = location.to_vec();
    settings.retain(|(tag, _)| *tag != Tag::new(b"opsz"));
    settings.push((Tag::new(b"opsz"), size));
    at_location(font, &settings)
}

pub(crate) fn at_location(font: &Font, settings: &[(Tag, f32)]) -> Font {
    let tables = font.tables();
    let avar = tables.avar().ok();
    if avar
        .as_ref()
        .is_some_and(|a| a.version() != MajorMinor::VERSION_1_0)
    {
        // Keep the dependency's inter-axis implementation. Callers supply the
        // complete user-space location, so adding opsz cannot erase weight.
        return font.instance_builder().variations(settings).build();
    }
    let mut coords = font.normalized_coords().to_vec();
    coords.resize(font.axes().len() as usize, NormalizedCoord::ZERO);
    for (tag, value) in settings {
        let Some(axis) = font.axes().get_by_tag(*tag) else {
            continue;
        };
        let mut coordinate = normalize(axis, *value);
        if let Some(Ok(map)) = avar
            .as_ref()
            .and_then(|a| a.axis_segment_maps().iter().nth(axis.index() as usize))
        {
            let points: Vec<_> = map
                .axis_value_maps()
                .iter()
                .map(|m| (m.from_coordinate().to_f64(), m.to_coordinate().to_f64()))
                .collect();
            coordinate = interpolate(coordinate, &points);
        }
        coords[axis.index() as usize] =
            NormalizedCoord::from_bits((coordinate.clamp(-1.0, 1.0) * 16384.0).round() as i16);
    }
    font.instance_builder().normalized_coords(coords).build()
}

fn normalize(axis: Axis, value: f32) -> f64 {
    let min = axis.min_value() as f64;
    let default = axis.default_value() as f64;
    let max = (axis.max_value() as f64).max(min);
    let value = (value as f64).clamp(min, max);
    // Avoid a 16.16 intermediate: 2/18 -> 7282/65536 -> 1821/16384
    // differs from CoreText's nearest 2.14 (1820/16384). A 1-unit difference
    // changes Inter's ceil64 width and can flip a wrap. Do not add epsilon.
    if value < default {
        (value - default) / (default - min)
    } else if value > default {
        (value - default) / (max - default)
    } else {
        0.0
    }
}

fn interpolate(value: f64, points: &[(f64, f64)]) -> f64 {
    if let Some(&(_, target)) = points.iter().find(|(source, _)| *source == value) {
        return target;
    }
    for pair in points.windows(2) {
        let [(a, x), (b, y)] = pair else {
            unreachable!()
        };
        if *a < value && value < *b {
            return x + (y - x) * (value - a) / (b - a);
        }
    }
    // Invalid or short maps retain the coordinate, rather than extrapolating
    // arbitrary malformed-font values. Valid avar maps always contain -1/0/1.
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optical_coordinate_avoids_double_rounding_and_maps_piecewise() {
        assert_eq!(((2.0_f64 / 18.0) * 16384.0).round(), 1820.0);
        assert_eq!(
            interpolate(-0.5, &[(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)]),
            -0.5
        );
        assert_eq!(interpolate(0.5, &[(0.0, 0.0), (1.0, 0.75)]), 0.375);
    }

    #[test]
    fn selection_clamps_and_preserves_every_other_normalized_coordinate() {
        // Synthetic metadata only: the existing OFL Noto wght becomes opsz,
        // retaining a separate wdth axis whose nondefault coordinate must survive.
        let mut bytes = include_bytes!("../../../renderer/fonts/NotoSans.ttf").to_vec();
        let count = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
        let record = bytes[12..12 + count * 16]
            .as_chunks::<16>()
            .0
            .iter()
            .find(|r| &r[..4] == b"fvar")
            .unwrap();
        let fvar = u32::from_be_bytes(record[8..12].try_into().unwrap()) as usize;
        let axis = fvar + u16::from_be_bytes([bytes[fvar + 4], bytes[fvar + 5]]) as usize;
        bytes[axis..axis + 4].copy_from_slice(b"opsz");
        let source: std::sync::Arc<dyn AsRef<[u8]> + Send + Sync> = std::sync::Arc::new(bytes);
        let font = Font::new(source, 0)
            .unwrap()
            .instance_builder()
            .normalized_coords([
                NormalizedCoord::from_bits(1024),
                NormalizedCoord::from_bits(-8192),
            ])
            .build();
        for (size, expected) in [(1.0, -16384), (400.0, 0), (1000.0, 16384)] {
            let instance = at_size(&font, size, &[]);
            assert_eq!(instance.normalized_coords()[0].to_bits(), expected);
            assert_eq!(instance.normalized_coords()[1].to_bits(), -8192);
        }
    }
}
