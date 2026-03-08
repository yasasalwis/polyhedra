//! Integration tests for CubeSdf.
//!
//! Tests the public API as a downstream user of the crate would — through
//! `_core::sdf::primitives::CubeSdf` and the `Sdf` trait.

use approx::assert_abs_diff_eq;
use _core::sdf::primitives::CubeSdf;
use _core::sdf::Sdf;
use glam::Vec3;

// ── Distance correctness ───────────────────────────────────────────────────

#[test]
fn cube_center_is_inside() {
    let c = CubeSdf::new(10.0, 10.0, 10.0);
    assert!(c.distance(Vec3::ZERO) < 0.0);
}

#[test]
fn cube_center_is_negative_half_min_extent() {
    // For a 10×10×10 cube the nearest face from the centre is 5 mm away.
    let c = CubeSdf::new(10.0, 10.0, 10.0);
    assert_abs_diff_eq!(c.distance(Vec3::ZERO), -5.0, epsilon = 1e-5);
}

#[test]
fn cube_face_centers_are_on_surface() {
    let c = CubeSdf::new(10.0, 10.0, 10.0);
    for face in [
        Vec3::new( 5.0,  0.0,  0.0),
        Vec3::new(-5.0,  0.0,  0.0),
        Vec3::new( 0.0,  5.0,  0.0),
        Vec3::new( 0.0, -5.0,  0.0),
        Vec3::new( 0.0,  0.0,  5.0),
        Vec3::new( 0.0,  0.0, -5.0),
    ] {
        let d = c.distance(face);
        assert!(d.abs() < 1e-5, "face point {face:?}: expected 0, got {d}");
    }
}

#[test]
fn cube_outside_axis_distance_is_correct() {
    let c = CubeSdf::new(10.0, 10.0, 10.0);
    // 3 mm past the +X face.
    assert_abs_diff_eq!(c.distance(Vec3::new(8.0, 0.0, 0.0)), 3.0, epsilon = 1e-5);
    // 7 mm past the -Y face.
    assert_abs_diff_eq!(c.distance(Vec3::new(0.0, -12.0, 0.0)), 7.0, epsilon = 1e-5);
}

#[test]
fn cube_outside_corner_distance_is_correct() {
    let c = CubeSdf::new(10.0, 10.0, 10.0);
    // Point (7, 7, 7): each axis is 2 mm past the face → distance = sqrt(12).
    let expected = (3.0_f32 * 2.0_f32.powi(2)).sqrt();
    assert_abs_diff_eq!(c.distance(Vec3::splat(7.0)), expected, epsilon = 1e-4);
}

#[test]
fn cube_inside_near_face_distance_is_correct() {
    let c = CubeSdf::new(10.0, 10.0, 10.0);
    // 1 mm from the +X face, inside.
    assert_abs_diff_eq!(c.distance(Vec3::new(4.0, 0.0, 0.0)), -1.0, epsilon = 1e-5);
}

// ── Non-uniform box ────────────────────────────────────────────────────────

#[test]
fn non_uniform_box_face_surfaces() {
    // 20 × 10 × 5 box.
    let b = CubeSdf::new(20.0, 10.0, 5.0);
    assert_abs_diff_eq!(b.distance(Vec3::new(10.0, 0.0, 0.0)), 0.0, epsilon = 1e-5);
    assert_abs_diff_eq!(b.distance(Vec3::new( 0.0, 5.0, 0.0)), 0.0, epsilon = 1e-5);
    assert_abs_diff_eq!(b.distance(Vec3::new( 0.0, 0.0, 2.5)), 0.0, epsilon = 1e-5);
}

#[test]
fn non_uniform_box_center_distance() {
    // Min half-extent is 2.5 → distance at centre = -2.5.
    let b = CubeSdf::new(20.0, 10.0, 5.0);
    assert_abs_diff_eq!(b.distance(Vec3::ZERO), -2.5, epsilon = 1e-5);
}

// ── Shape type ─────────────────────────────────────────────────────────────

#[test]
fn shape_type_cube_id_is_zero() {
    use _core::shape_type::ShapeType;
    assert_eq!(ShapeType::try_from(0u32).unwrap(), ShapeType::Cube);
}

#[test]
fn shape_type_all_ids_convert() {
    use _core::shape_type::ShapeType;
    for id in 0u32..=6 {
        assert!(ShapeType::try_from(id).is_ok(), "id {id} should be a valid ShapeType");
    }
}

#[test]
fn shape_type_unknown_id_errors() {
    use _core::shape_type::ShapeType;
    assert!(ShapeType::try_from(99u32).is_err());
}

// ── SdfNode (trait object) ────────────────────────────────────────────────

#[test]
fn cube_as_boxed_sdf_node() {
    use _core::sdf::SdfNode;
    let node: SdfNode = Box::new(CubeSdf::new(10.0, 10.0, 10.0));
    assert!(node.distance(Vec3::ZERO) < 0.0);
    assert!(node.distance(Vec3::new(10.0, 10.0, 10.0)) > 0.0);
}

// ── Union of two cubes ────────────────────────────────────────────────────

#[test]
fn union_of_two_cubes_covers_both_interiors() {
    use _core::sdf::operations::UnionNode;
    use _core::sdf::transform::Translate;
    use _core::sdf::SdfNode;

    let a: SdfNode = Box::new(CubeSdf::new(10.0, 10.0, 10.0));
    let b: SdfNode = Box::new(CubeSdf::new(10.0, 10.0, 10.0));
    // Shift b by 8 mm on X so they overlap.
    let b_shifted: SdfNode = Box::new(Translate {
        inner:  b,
        offset: Vec3::new(8.0, 0.0, 0.0),
    });
    let union_node: SdfNode = Box::new(UnionNode { a, b: b_shifted });

    // Centre of A (origin) should be inside the union.
    assert!(union_node.distance(Vec3::ZERO) < 0.0);
    // Centre of B (8, 0, 0) should be inside the union.
    assert!(union_node.distance(Vec3::new(8.0, 0.0, 0.0)) < 0.0);
    // Far away point should be outside.
    assert!(union_node.distance(Vec3::new(100.0, 0.0, 0.0)) > 0.0);
}
