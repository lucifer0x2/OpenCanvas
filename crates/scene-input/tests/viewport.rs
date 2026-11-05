use opencanvas_scene_input::check_viewport;

fn rejected(width: f64, height: f64, scale: f64) {
    let error = check_viewport(width, height, scale).unwrap_err();
    assert_eq!(error.code, "INVALID_REQUEST");
    assert_eq!(error.message, "Viewport or scale exceeds the render budget");
    assert!(!error.retryable);
}

#[test]
fn dimensions_must_be_finite_positive_and_within_each_side_limit() {
    for invalid in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -1.0,
        100_000.000_001,
    ] {
        rejected(invalid, 1.0, 1.0);
        rejected(1.0, invalid, 1.0);
    }
    for valid in [f64::MIN_POSITIVE, 1.0, 100_000.0] {
        check_viewport(valid, 1.0, 1.0).unwrap();
        check_viewport(1.0, valid, 1.0).unwrap();
    }
}

#[test]
fn scale_must_be_finite_positive_and_at_most_four() {
    for invalid in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -1.0,
        4.000_001,
    ] {
        rejected(1.0, 1.0, invalid);
    }
    for valid in [f64::MIN_POSITIVE, 0.5, 1.0, 2.0, 4.0] {
        check_viewport(1.0, 1.0, valid).unwrap();
    }
}

#[test]
fn scaled_pixel_budget_accepts_exact_boundary_and_rejects_the_next_size() {
    for (width, height, scale) in [
        (4000.0, 4000.0, 1.0),
        (2000.0, 2000.0, 2.0),
        (1000.0, 1000.0, 4.0),
        (8000.0, 8000.0, 0.5),
        (100_000.0, 160.0, 1.0),
    ] {
        check_viewport(width, height, scale).unwrap();
        rejected(width, height + 0.001, scale);
        rejected(height + 0.001, width, scale);
    }
}
