//! B05: one `fit` vocabulary from validator to scene. The renderer side is
//! renderer/image-fit.test.mjs; the scene side through a real render is
//! crates/server/tests/image_fit.rs.
mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::json;

fn image(asset: &str, id: &str, fit: &str) -> Operation {
    create(
        id,
        NodeKind::Image,
        Some("frame"),
        json!({"assetId":asset,"width":40,"height":40,"fit":fit}),
    )
}

#[test]
fn layout_accepts_exactly_the_validator_fits() {
    assert_eq!(IMAGE_FITS, opencanvas_layout::IMAGE_FITS);
}

#[test]
fn every_fit_is_accepted_and_resolves_to_its_authored_name() {
    let (_temp, _store, p) = setup();
    let asset = p
        .import_asset(asset_request(&p, 0, "asset"))
        .unwrap()
        .asset
        .id;
    let mut ops = initial();
    ops.extend(IMAGE_FITS.map(|fit| image(&asset, fit, fit)));
    p.apply_batch(request(&p, 1, "images", ops)).unwrap();
    let resolved = resolved_nodes(&p.snapshot().unwrap()).unwrap();
    for fit in IMAGE_FITS {
        let node = resolved.iter().find(|n| n.id == fit).unwrap();
        assert_eq!(
            node.props["fit"], fit,
            "resolved fit keeps the authored name"
        );
    }
}

#[test]
fn other_names_fail_with_a_pointer_to_the_supported_ones() {
    let (_temp, _store, p) = setup();
    let asset = p
        .import_asset(asset_request(&p, 0, "asset"))
        .unwrap()
        .asset
        .id;
    for (fit, hint) in [
        ("fill", "Use \"fit\" to stretch"),
        ("stretch", "Use \"fit\" to stretch"),
        ("none", "Use \"contain\""),
        ("tile", "fit accepts fit (stretch to bounds), contain"),
    ] {
        let mut ops = initial();
        ops.push(image(&asset, "photo", fit));
        let error = p.apply_batch(request(&p, 1, fit, ops)).unwrap_err();
        assert_eq!(error.code, "VALIDATION_FAILED", "{fit}");
        assert_eq!(error.field.as_deref(), Some("fit"), "{fit}");
        assert!(error.message.contains(hint), "{fit}: {}", error.message);
    }
}

#[test]
fn published_fit_reference_names_every_fit() {
    let spec = prop_spec("fit").unwrap();
    for fit in IMAGE_FITS {
        assert!(spec.value.contains(&format!("\"{fit}\"")), "{fit}");
    }
}
