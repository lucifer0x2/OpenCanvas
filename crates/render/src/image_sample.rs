//! Canvas/Skia low-quality image filtering: four-bit bilinear weights in premul.
//! Coordinates refer to pixel centers (integer zero is the first source pixel).
use tiny_skia::Pixmap;

pub(super) fn bilinear(image: &Pixmap, x: f64, y: f64, alpha: f32, reversed: [bool; 2]) -> [u8; 4] {
    let axis = |p: f64, len: u32, reversed: bool| {
        let low = p.floor();
        let a = low.clamp(0.0, (len - 1) as f64) as usize;
        let b = (low + 1.0).clamp(0.0, (len - 1) as f64) as usize;
        let weight = (p - low) * 16.0;
        (
            a,
            b,
            if reversed {
                weight.ceil()
            } else {
                weight.floor()
            } as u32,
        )
    };
    let (x0, x1, fx) = axis(x, image.width(), reversed[0]);
    let (y0, y1, fy) = axis(y, image.height(), reversed[1]);
    let at = |x, y, c| u32::from(image.data()[(y * image.width() as usize + x) * 4 + c]);
    // Filtering truncates once after both axes, not after each intermediate
    // lerp. This retains transparent-edge premultiplied color without halos.
    std::array::from_fn(|c| {
        let v = (at(x0, y0, c) * (16 - fx) * (16 - fy)
            + at(x1, y0, c) * fx * (16 - fy)
            + at(x0, y1, c) * (16 - fx) * fy
            + at(x1, y1, c) * fx * fy)
            >> 8;
        (v as f32 * alpha).round() as u8
    })
}
