use crate::{
    transform_text, wrap_text, Font, MeasureRequest, Measurement, TextError, TextLine,
    MAX_TEXT_BYTES,
};
use std::collections::HashMap;

fn ceil64(value: f64) -> f32 {
    (value.max(0.0) * 64.0).ceil() as f32 / 64.0
}
fn round64(value: f64) -> f32 {
    (value * 64.0 + 0.5).floor() as f32 / 64.0
}

pub fn measure(font: &Font, r: &MeasureRequest) -> Result<Measurement, TextError> {
    if r.text.len() > MAX_TEXT_BYTES {
        return Err(TextError::TextLimit);
    }
    if !r.line_height.is_finite()
        || !(0.0..=10000.0).contains(&r.line_height)
        || r.width.is_some_and(|w| !w.is_finite())
    {
        return Err(TextError::InvalidRequest);
    }
    let metrics = font.metrics(r.font_size)?;
    let spacing = r.letter_spacing.unwrap_or(0.0);
    // Validate tracking even for empty text.
    font.positioned_run("", r.font_size, spacing)?;
    let rendered = transform_text(&r.text, r.text_transform.as_deref())?;
    let max_width = r.width.map_or(f32::INFINITY, |w| w.max(0.0));
    let mut widths = HashMap::<String, f32>::new();
    let mut error = None;
    let texts = wrap_text(&rendered, |text| {
        if error.is_some() {
            return true;
        }
        let width = if let Some(width) = widths.get(text) {
            *width
        } else {
            match font.positioned_run(text, r.font_size, spacing) {
                Ok(run) => {
                    let width = ceil64(run.advance);
                    // Bound retained candidate strings; emergency splitting otherwise caches
                    // every growing prefix and takes quadratic memory for a long word.
                    if widths.len() < 256 && text.len() <= 4096 {
                        widths.insert(text.to_owned(), width);
                    }
                    width
                }
                Err(err) => {
                    error = Some(err);
                    return true;
                }
            }
        };
        width <= max_width
    });
    if let Some(err) = error {
        return Err(err);
    }
    let baseline = metrics.ascent as f64
        + ((r.line_height as f64 - metrics.ascent as f64 - metrics.descent as f64) / 2.0).floor();
    let mut measurement = Measurement {
        height: round64(texts.len() as f64 * r.line_height as f64),
        ascent: metrics.ascent,
        missing: font.missing_codepoints(&rendered)?,
        ..Measurement::default()
    };
    for (index, text) in texts.into_iter().enumerate() {
        let run = font.positioned_run(&text, r.font_size, spacing)?;
        let width = ceil64(run.advance);
        let ink_bounds = Some(font.ink_bounds(&run, r.font_size)?);
        measurement.width = measurement.width.max(width);
        measurement.lines.push(TextLine {
            text,
            x: 0.0,
            y: round64(index as f64 * r.line_height as f64 + baseline),
            width,
            ink_bounds,
        });
    }
    Ok(measurement)
}
