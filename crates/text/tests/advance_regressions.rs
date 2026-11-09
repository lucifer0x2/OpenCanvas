mod support;
use support::*;

#[test]
fn blink_nominal_truncation_retains_recorded_spike_ceil64_widths() {
    // Existing Chromium measurements in spikes/rust-export/data/widths_report.json.
    // This is a fixed regression, not a new claim of current-browser corpus parity.
    let cases = [
        ("Thiết kế giao diện người dùng cần nhất quán về khoảng cách, màu sắc và kiểu chữ trên mọi màn hình.", 24.0, 1150.366943359375_f64),
        ("Thiết kế giao diện người dùng cần nhất quán về khoảng cách, màu sắc và kiểu chữ trên mọi màn hình.", 16.0, 766.9110107421875),
        ("Văn bản tiếng Việt có dấu thanh và dấu mũ chồng lên nhau: ặ ậ ệ ộ ợ ự ỵ — đo đúng là quan trọng.", 24.0, 1110.094970703125),
        ("The quick brown fox jumps over the lazy dog while the export pipeline renders every frame.", 16.0, 685.5029907226562),
        ("Measured widths are rounded to a 1/64 grid so layout converges in a bounded number of passes.", 24.0, 1097.20703125),
    ];
    let font = font();
    for (text, size, chrome) in cases {
        let run = font.positioned_run(text, size, 0.0).unwrap();
        assert!(
            (run.advance - chrome).abs() <= 0.0000611,
            "{text}: {} != {chrome}",
            run.advance
        );
        let mut r = request(text);
        r.font_size = size;
        assert_eq!(
            set().measure(&r).unwrap().width,
            (chrome * 64.0).ceil() as f32 / 64.0
        );
    }
}

#[test]
fn zero_width_default_ignorables_do_not_gain_nominal_rounding_deltas() {
    let font = font();
    let plain = font.positioned_run("xx", 16.0, 0.0).unwrap();
    for c in ['\u{2060}', '\u{feff}', '\u{200b}'] {
        let run = font.positioned_run(&format!("x{c}x"), 16.0, 0.0).unwrap();
        assert_eq!(run.advance, plain.advance);
    }
}
