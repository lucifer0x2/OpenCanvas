use opencanvas_layout::Props;
use serde_json::json;

/// Navigation props are authored data with no geometry: layout ignores them.
#[test]
fn layout_props_ignore_links_and_flow_start() {
    let plain = json!({"name":"Home","width":390,"height":844,"fill":"#ffffff"});
    let mut linked = plain.clone();
    linked["links"] = json!([{"to":"search"},{"on":"hover","action":"overlay","to":"menu"}]);
    linked["flowStart"] = json!("Main");
    let plain: Props = serde_json::from_value(plain).unwrap();
    let linked: Props = serde_json::from_value(linked).unwrap();
    assert_eq!(format!("{plain:?}"), format!("{linked:?}"));
}
