//! Measurement adapters share request keys; neither adapter performs I/O itself.
use crate::{LayoutError, MeasureCache, MeasureRequest, Measurement};

/// Supplies text metrics and prewrapped lines synchronously, from prepared resources.
///
/// Requests use the existing renderer-versioned key and quantized width contract.
/// A layout invocation can request several constraints for the same text, including
/// its final box width. Errors abort computation without publishing a partial scene.
/// Error codes/messages (including `JOB_TIMEOUT` and `RESOURCE_NOT_READY`) are
/// propagated unchanged, allowing callers to enforce their own readiness/budget.
pub trait TextMeasurer {
    fn measure(&mut self, request: &MeasureRequest) -> Result<Measurement, LayoutError>;
}

pub(crate) trait MeasurementSource {
    fn get(&mut self, request: &MeasureRequest) -> Result<Option<&Measurement>, LayoutError>;
}

pub(crate) struct CachedMeasurements<'a>(pub &'a MeasureCache);

impl MeasurementSource for CachedMeasurements<'_> {
    fn get(&mut self, request: &MeasureRequest) -> Result<Option<&Measurement>, LayoutError> {
        Ok(self.0.get(&request.key))
    }
}

/// This cache lives for exactly one compute call, never across input revisions.
pub(crate) struct SynchronousMeasurements<'a, M: ?Sized> {
    pub measurer: &'a mut M,
    pub cache: MeasureCache,
}

impl<M: TextMeasurer + ?Sized> MeasurementSource for SynchronousMeasurements<'_, M> {
    fn get(&mut self, request: &MeasureRequest) -> Result<Option<&Measurement>, LayoutError> {
        use std::collections::btree_map::Entry;
        let measurement = match self.cache.entry(request.key.clone()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(self.measurer.measure(request)?),
        };
        Ok(Some(measurement))
    }
}
