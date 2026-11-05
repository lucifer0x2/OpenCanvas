//! Caller-owned font measurement adapter shared by native and WASM layout.
use crate::FontSet;
use opencanvas_layout::{LayoutError, MeasureRequest, Measurement, TextMeasurer};

/// Measures exact font identities, with optional caller cancellation/deadline checks.
/// A check runs before and after each successful measurement. No clock or runtime
/// is required; a browser caller may use [`Self::new`] without a callback.
pub struct FontSetMeasurer<'a, C = fn() -> Result<(), LayoutError>> {
    fonts: &'a FontSet,
    check: C,
    measured: usize,
}

impl<'a> FontSetMeasurer<'a> {
    pub fn new(fonts: &'a FontSet) -> Self {
        Self::with_check(fonts, || Ok(()))
    }
}

impl<'a, C: FnMut() -> Result<(), LayoutError>> FontSetMeasurer<'a, C> {
    pub fn with_check(fonts: &'a FontSet, check: C) -> Self {
        Self {
            fonts,
            check,
            measured: 0,
        }
    }

    /// Successful shaping calls, including a result rejected by the trailing check.
    pub fn measured(&self) -> usize {
        self.measured
    }
}

impl<C: FnMut() -> Result<(), LayoutError>> TextMeasurer for FontSetMeasurer<'_, C> {
    fn measure(&mut self, request: &MeasureRequest) -> Result<Measurement, LayoutError> {
        (self.check)()?;
        let measurement = self.fonts.measure(request).map_err(|error| LayoutError {
            code: error.code().into(),
            message: "Text resources or measurement are invalid".into(),
        })?;
        self.measured += 1;
        (self.check)()?;
        Ok(measurement)
    }
}
