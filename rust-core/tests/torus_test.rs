//! Integration tests for TorusSdf.

use approx::assert_abs_diff_eq;
use _core::sdf::primitives::TorusSdf;
use _core::sdf::Sdf;
use glam::Vec3;

/// R=5, r=2.
fn torus() -> TorusSdf { TorusSdf::new(5.0, 2.0) }

// ── Interior / exterior ───────────────────────────────────────────────────

#[test]
fn origin_in_hole_is_outside() {
    // d(origin) = 5 - 2 = 3.
    assert_abs_diff_eq!(torus().distance(Vec3::ZERO), 3.0, epsilon = 1e-5);
}

#[test]
fn tube_centre_is_inside() {
    assert_abs_diff_eq!(torus().distance(Vec3::new(5.0, 0.0, 0.0)), -2.0, epsilon = 1e-5);
}

// ── Surface ───────────────────────────────────────────────────────────────

#[test]
fn outer_surface_zero() {
    assert_abs_diff_eq!(torus().distance(Vec3::new(7.0, 0.0, 0.0)), 0.0, epsilon = 1e-5);
}

#[test]
fn inner_surface_zero() {
    assert_abs_diff_eq!(torus().distance(Vec3::new(3.0, 0.0, 0.0)), 0.0, epsilon = 1e-5);
}

#[test]
fn top_of_tube_zero() {
    assert_abs_diff_eq!(torus().distance(Vec3::new(5.0, 0.0, 2.0)), 0.0, epsilon = 1e-5);
}

#[test]
fn surface_rotationally_symmetric() {
    let t = torus();
    for deg in [0.0_f32, 60.0, 120.0, 180.0, 240.0, 300.0] {
        let a = deg.to_radians();
        let p = Vec3::new(7.0 * a.cos(), 7.0 * a.sin(), 0.0);
        let d = t.distance(p);
        assert!(d.abs() < 1e-5, "{deg}°: expected ~0, got {d}");
    }
}

// ── Exterior distances ────────────────────────────────────────────────────

#[test]
fn above_tube_distance() {
    assert_abs_diff_eq!(torus().distance(Vec3::new(5.0, 0.0, 5.0)), 3.0, epsilon = 1e-5);
}

#[test]
fn past_outer_rim_distance() {
    assert_abs_diff_eq!(torus().distance(Vec3::new(9.0, 0.0, 0.0)), 2.0, epsilon = 1e-5);
}

// ── Shape-type mapping ────────────────────────────────────────────────────

#[test]
fn shape_type_torus_id_is_four() {
    use _core::shape_type::ShapeType;
    assert_eq!(ShapeType::try_from(4u32).unwrap(), ShapeType::Torus);
}

// ── Trait object ──────────────────────────────────────────────────────────

#[test]
fn torus_as_sdf_node() {
    use _core::sdf::SdfNode;
    let node: SdfNode = Box::new(TorusSdf::new(5.0, 2.0));
    assert!(node.distance(Vec3::new(5.0, 0.0, 0.0)) < 0.0);
    assert!(node.distance(Vec3::ZERO) > 0.0);
}

// ── Torus intersected with cube (half-torus) ──────────────────────────────

#[test]
fn torus_half_via_intersection() {
    use _core::sdf::operations::IntersectionNode;
    use _core::sdf::primitives::CubeSdf;
    use _core::sdf::SdfNode;

    // Keep only the +X half of the torus.
    let torus: SdfNode = Box::new(TorusSdf::new(5.0, 2.0));
    // A wide box covering only x >= 0 — translated +5 on X so it starts at x=0.
    let half_space: SdfNode = Box::new(_core::sdf::transform::Translate {
        inner:  Box::new(CubeSdf::new(20.0, 20.0, 20.0)),
        offset: glam::Vec3::new(10.0, 0.0, 0.0),
    });
    let half_torus: SdfNode = Box::new(IntersectionNode { a: torus, b: half_space });

    // Tube centre in +X half → inside half-torus.
    assert!(half_torus.distance(Vec3::new(5.0, 0.0, 0.0)) < 0.0);
    // Tube centre in -X half → outside (clipped).
    assert!(half_torus.distance(Vec3::new(-5.0, 0.0, 0.0)) > 0.0);
}

// ── Smooth union: torus + sphere (blobby merge) ───────────────────────────

#[test]
fn torus_smooth_union_with_sphere() {
    use _core::sdf::operations::SmoothUnionNode;
    use _core::sdf::primitives::SphereSdf;
    use _core::sdf::SdfNode;

    let t: SdfNode = Box::new(TorusSdf::new(5.0, 1.5));
    let s: SdfNode = Box::new(SphereSdf::new(3.0));
    let blob: SdfNode = Box::new(SmoothUnionNode { a: t, b: s, k: 1.5 });

    // Inside the sphere (near origin) → inside the blob.
    assert!(blob.distance(Vec3::new(2.0, 0.0, 0.0)) < 0.0);
    // Tube centre of torus → inside.
    assert!(blob.distance(Vec3::new(5.0, 0.0, 0.0)) < 0.0);
    // Far away → outside.
    assert!(blob.distance(Vec3::new(50.0, 0.0, 0.0)) > 0.0);
}
