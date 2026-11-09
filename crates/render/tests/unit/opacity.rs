use super::*;

#[test]
fn overlapping_children_composite_once_and_budget_fold_is_explicit() {
    let mut parent = node("p", 0.0, 0.0, 64.0, 64.0, "transparent");
    parent.opacity = 0.5;
    let mut a = node("a", 10.0, 10.0, 30.0, 30.0, "#f00");
    a.parent_id = Some("p".into());
    let mut b = node("b", 20.0, 20.0, 30.0, 30.0, "#f00");
    b.parent_id = Some("p".into());
    let s = scene(vec![parent, a, b]);
    let r = render(&s);
    assert_eq!(r.draw.opacity_layers, 1);
    assert_eq!(r.draw.opacity_layer_fallbacks, 0);
    assert_eq!(pixel(&r, 25, 25)[3], pixel(&r, 15, 15)[3]);
    let o = RenderOptions {
        max_layer_pixels: 0,
        ..RenderOptions::default()
    };
    let r = crate::render(&s, &Resources::default(), &o).unwrap();
    assert_eq!(r.draw.opacity_layers, 0);
    assert_eq!(r.draw.opacity_layer_fallbacks, 1);
    assert!(pixel(&r, 25, 25)[3] > pixel(&r, 15, 15)[3]);
}
#[test]
fn live_not_cumulative_layer_budget_and_integer_offsets() {
    let mut nodes = Vec::new();
    for (id, x) in [("a", 3.25), ("b", 35.25)] {
        let mut p = node(id, x, 4.5, 20.0, 30.0, "#f00");
        p.opacity = 0.5;
        let mut c = node(&format!("{id}-c"), x + 5.0, 8.5, 10.0, 10.0, "#00f");
        c.parent_id = Some(id.into());
        nodes.push(p);
        nodes.push(c);
    }
    let o = RenderOptions {
        max_layer_pixels: 800,
        ..RenderOptions::default()
    };
    let r = crate::render(&scene(nodes), &Resources::default(), &o).unwrap();
    assert_eq!(r.draw.opacity_layers, 2);
    assert_eq!(r.draw.opacity_layer_fallbacks, 0);
}
#[test]
fn nested_live_budget_counts_simultaneous_layers_only() {
    let mut a = node("a", 0.0, 0.0, 50.0, 50.0, "#fff");
    a.opacity = 0.5;
    let mut b = node("b", 10.0, 10.0, 30.0, 30.0, "#f00");
    b.opacity = 0.5;
    b.parent_id = Some("a".into());
    let mut c = node("c", 15.0, 15.0, 20.0, 20.0, "#00f");
    c.parent_id = Some("b".into());
    let o = RenderOptions {
        max_layer_pixels: 3000,
        ..RenderOptions::default()
    };
    let r = crate::render(&scene(vec![a, b, c]), &Resources::default(), &o).unwrap();
    assert_eq!(r.draw.opacity_layers, 1);
    assert_eq!(r.draw.opacity_layer_fallbacks, 1);
}
#[test]
fn single_paint_needs_no_layer_and_empty_clip_cannot_leak() {
    let mut a = node("a", 0.0, 0.0, 40.0, 40.0, "transparent");
    a.opacity = 0.5;
    let mut b = node("b", 10.0, 10.0, 10.0, 10.0, "#f00");
    b.parent_id = Some("a".into());
    let r = render(&scene(vec![a.clone(), b.clone()]));
    assert_eq!(r.draw.opacity_layers, 0);
    a.clip = true;
    a.bounds.width = 0.0;
    let r = render(&scene(vec![a, b]));
    assert!(r.rgba.iter().all(|v| *v == 0));
}
