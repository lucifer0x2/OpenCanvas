use super::*;
use opencanvas_layout::{Paint, SceneStroke, Shadow, Sides};

#[test]
fn corner_reduction_is_css_shared_factor_and_inner_radii_are_elliptical() {
    let mut n = node("n", 5.0, 5.0, 60.0, 30.0, "#f00");
    n.radius = Sides::Each([40.0, 20.0, 0.0, 10.0]);
    let r = crate::geometry::corners(&n);
    assert_eq!(r, [[24.0, 24.0], [12.0, 12.0], [0.0, 0.0], [6.0, 6.0]]);
    n.stroke = Some(SceneStroke {
        width: [3.0, 7.0, 5.0, 11.0],
        color: ["#000".into(), "#000".into(), "#000".into(), "#000".into()],
        align: "inside".into(),
    });
    let e = crate::stroke::edges(&n).unwrap();
    assert_eq!(e.inner_r[0], [13.0, 21.0]);
}
#[test]
fn rounded_fill_and_aa_clip_cut_descendants_but_not_own_outside_stroke() {
    let mut p = node("p", 10.0, 10.0, 30.0, 30.0, "#fff");
    p.clip = true;
    p.radius = Sides::All(10.0);
    p.stroke = Some(SceneStroke {
        width: [4.0; 4],
        color: ["#00f".into(), "#00f".into(), "#00f".into(), "#00f".into()],
        align: "outside".into(),
    });
    let mut child = node("c", 0.0, 0.0, 64.0, 64.0, "#f00");
    child.parent_id = Some("p".into());
    let r = render(&scene(vec![p, child]));
    assert_eq!(pixel(&r, 25, 7), [0, 0, 255, 255]);
    assert_eq!(pixel(&r, 25, 15), [255, 0, 0, 255]);
    assert_eq!(pixel(&r, 0, 0), [0; 4]);
    assert!(r
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .any(|p| p[3] > 0 && p[3] < 255));
}
#[test]
fn per_side_color_and_post_subtree_stroke_order() {
    let mut p = node("p", 8.0, 8.0, 40.0, 40.0, "#fff");
    p.stroke = Some(SceneStroke {
        width: [6.0; 4],
        color: ["#f00".into(), "#0f0".into(), "#00f".into(), "#ff0".into()],
        align: "inside".into(),
    });
    let mut c = node("c", 8.0, 8.0, 40.0, 40.0, "#000");
    c.parent_id = Some("p".into());
    let r = render(&scene(vec![p, c]));
    for (x, y, v) in [
        (28, 10, [255, 0, 0, 255]),
        (45, 28, [0, 255, 0, 255]),
        (28, 45, [0, 0, 255, 255]),
        (10, 28, [255, 255, 0, 255]),
    ] {
        assert_eq!(pixel(&r, x, y), v);
    }
    assert_eq!(pixel(&r, 28, 28), [0, 0, 0, 255]);
}
#[test]
fn gradients_keep_hard_stops_and_premultiplied_srgb() {
    let mut n = node("n", 0.0, 0.0, 64.0, 64.0, "transparent");
    n.fill=serde_json::from_value(json!({"type":"linear","angle":90,"stops":[[0,"#f00"],[0.5,"#f00"],[0.5,"#00f"],[1,"#00f"]]})).unwrap();
    let r = render(&scene(vec![n.clone()]));
    assert_eq!(pixel(&r, 31, 20), [255, 0, 0, 255]);
    assert_eq!(pixel(&r, 32, 20), [0, 0, 255, 255]);
    n.fill = serde_json::from_value(
        json!({"type":"linear","angle":90,"stops":[[0,"#f000"],[1,"#00f"]]}),
    )
    .unwrap();
    let r = render(&scene(vec![n.clone()]));
    let p = pixel(&r, 32, 20);
    assert_eq!(p[0], 0);
    assert!((125..=132).contains(&p[2]));
    assert_eq!(p[2], p[3]);
    n.fill=serde_json::from_value(json!({"type":"radial","center":[0.5,0.5],"radius":[30,10],"stops":[[0,"#fff"],[1,"#000"]]})).unwrap();
    let r = render(&scene(vec![n]));
    assert!(pixel(&r, 32, 32)[0] > pixel(&r, 32, 41)[0]);
}
#[test]
fn outer_and_inset_shadow_spread_blur_are_visible_and_bounded() {
    let mut n = node("n", 20.0, 20.0, 24.0, 24.0, "#fff");
    n.shadow = vec![Shadow {
        x: 5.0,
        y: 0.0,
        blur: 0.0,
        spread: 2.0,
        color: "#f00".into(),
        inset: false,
    }];
    let r = render(&scene(vec![n.clone()]));
    assert_eq!(pixel(&r, 48, 30), [255, 0, 0, 255]);
    assert_eq!(pixel(&r, 30, 30), [255; 4]);
    n.shadow[0].blur = 6.0;
    let r = render(&scene(vec![n.clone()]));
    assert!(pixel(&r, 51, 30)[3] > 0 && pixel(&r, 51, 30)[3] < 255);
    n.shadow[0] = Shadow {
        x: 0.0,
        y: 0.0,
        blur: 0.0,
        spread: 4.0,
        color: "#000".into(),
        inset: true,
    };
    let r = render(&scene(vec![n.clone()]));
    assert_eq!(pixel(&r, 21, 30), [0, 0, 0, 255]);
    assert_eq!(pixel(&r, 30, 30), [255; 4]);
    n.shadow[0].blur = 4.0;
    let r = render(&scene(vec![n.clone()]));
    // The exterior is an infinite complement, not zero-valued scratch padding.
    assert!(pixel(&r, 20, 30)[0] < 30);
    n.shadow[0].blur = 0.0;
    n.fill = Paint::from("transparent");
    n.shadow[0].inset = false;
    let r = render(&scene(vec![n]));
    assert_eq!(pixel(&r, 30, 30), [0; 4]);
}
