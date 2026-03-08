//! Integration tests for SphereSdf.

use _core::sdf::Sdf;
use _core::sdf::primitives::SphereSdf;
use approx::assert_abs_diff_eq;
use glam::Vec3;

fn sphere() -> SphereSdf {
    SphereSdf::new(5.0)
}

// ── Interior ──────────────────────────────────────────────────────────────

#[test]
fn sphere_center_inside() {
    assert!(sphere().distance(Vec3::ZERO) < 0.0);
}

#[test]
fn sphere_center_distance() {
    assert_abs_diff_eq!(sphere().distance(Vec3::ZERO), -5.0, epsilon = 1e-5);
}

// ── Surface ───────────────────────────────────────────────────────────────

#[test]
fn sphere_surface_on_axes() {
    let s = sphere();
    for p in [
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(-5.0, 0.0, 0.0),
        Vec3::new(0.0, 5.0, 0.0),
        Vec3::new(0.0, -5.0, 0.0),
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(0.0, 0.0, -5.0),
    ] {
        let d = s.distance(p);
        assert!(d.abs() < 1e-5, "{p:?}: expected ~0, got {d}");
    }
}

#[test]
fn sphere_surface_on_diagonal() {
    let p = Vec3::splat(1.0).normalize() * 5.0;
    assert_abs_diff_eq!(sphere().distance(p), 0.0, epsilon = 1e-5);
}

// ── Exterior ─────────────────────────────────────────────────────────────

#[test]
fn sphere_outside_distance() {
    assert_abs_diff_eq!(
        sphere().distance(Vec3::new(8.0, 0.0, 0.0)),
        3.0,
        epsilon = 1e-5
    );
}

#[test]
fn sphere_isotropic() {
    let s = sphere();
    let r = 9.0_f32;
    let expected = r - 5.0;
    for p in [
        Vec3::new(r, 0.0, 0.0),
        Vec3::new(0.0, r, 0.0),
        Vec3::new(0.0, 0.0, r),
        Vec3::new(-r, 0.0, 0.0),
    ] {
        assert_abs_diff_eq!(s.distance(p), expected, epsilon = 1e-5);
    }
}

// ── Shape-type mapping ────────────────────────────────────────────────────

#[test]
fn shape_type_sphere_id_is_two() {
    use _core::shape_type::ShapeType;
    assert_eq!(ShapeType::try_from(2u32).unwrap(), ShapeType::Sphere);
}

// ── Trait object ──────────────────────────────────────────────────────────

#[test]
fn sphere_as_sdf_node() {
    use _core::sdf::SdfNode;
    let node: SdfNode = Box::new(SphereSdf::new(5.0));
    assert!(node.distance(Vec3::ZERO) < 0.0);
    assert!(node.distance(Vec3::new(10.0, 0.0, 0.0)) > 0.0);
}

// ── Smooth union: sphere blended with cube ────────────────────────────────

#[test]
fn sphere_smooth_union_with_cube() {
    use _core::sdf::SdfNode;
    use _core::sdf::operations::SmoothUnionNode;
    use _core::sdf::primitives::CubeSdf;

    let cube: SdfNode = Box::new(CubeSdf::new(10.0, 10.0, 10.0));
    let sphere: SdfNode = Box::new(SphereSdf::new(4.0));
    let blended: SdfNode = Box::new(SmoothUnionNode {
        a: cube,
        b: sphere,
        k: 2.0,
    });

    // Both origins are at (0,0,0) — point is inside both → inside result.
    assert!(blended.distance(Vec3::ZERO) < 0.0);
    // Far outside → positive.
    assert!(blended.distance(Vec3::new(50.0, 0.0, 0.0)) > 0.0);
}

// ── Intersection: sphere clipped by cube ─────────────────────────────────

#[test]
fn sphere_intersect_cube_clips_corners() {
    use _core::sdf::SdfNode;
    use _core::sdf::operations::IntersectionNode;
    use _core::sdf::primitives::CubeSdf;

    // r=8 sphere intersected with a 10×10×10 cube.
    // The sphere corners (beyond r=5 along the diagonal) get clipped.
    let sphere: SdfNode = Box::new(SphereSdf::new(8.0));
    let cube: SdfNode = Box::new(CubeSdf::new(10.0, 10.0, 10.0));
    let clipped: SdfNode = Box::new(IntersectionNode { a: sphere, b: cube });

    // At origin: inside both → inside intersection.
    assert!(clipped.distance(Vec3::ZERO) < 0.0);
    // At cube corner (7.07, 7.07, 7.07) — inside sphere (|p|≈12.2 > 8), outside cube → outside.
    assert!(clipped.distance(Vec3::splat(7.07)) > 0.0);
}
