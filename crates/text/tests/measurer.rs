mod support;
use opencanvas_layout::{LayoutError, TextMeasurer};
use opencanvas_text::{FontSet, FontSetMeasurer};

#[test]
fn adapter_preserves_exact_measurements_counts_and_safe_error_mapping() {
    let fonts = support::set();
    let request = support::request("Tiếng Việt");
    let expected = fonts.measure(&request).unwrap();
    let mut adapter = FontSetMeasurer::new(&fonts);
    let actual = adapter.measure(&request).unwrap();
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert_eq!(adapter.measured(), 1);
    let empty = FontSet::new();
    let mut adapter = FontSetMeasurer::new(&empty);
    let error = adapter.measure(&request).unwrap_err();
    assert_eq!(error.code, "FONT_MISSING");
    assert_eq!(
        error.message,
        "Native text resources or measurement are invalid"
    );
    assert_eq!(adapter.measured(), 0);
}

#[test]
fn caller_checks_preserve_pre_and_post_measure_cancellation() {
    let fonts = support::set();
    let request = support::request("Cancel");
    for fail_at in [1, 2] {
        let mut checks = 0;
        let mut adapter = FontSetMeasurer::with_check(&fonts, || {
            checks += 1;
            if checks == fail_at {
                Err(LayoutError {
                    code: "JOB_CANCELLED".into(),
                    message: "Caller cancelled".into(),
                })
            } else {
                Ok(())
            }
        });
        let error = adapter.measure(&request).unwrap_err();
        assert_eq!(error.code, "JOB_CANCELLED");
        assert_eq!(error.message, "Caller cancelled");
        assert_eq!(adapter.measured(), usize::from(fail_at == 2));
    }
}
