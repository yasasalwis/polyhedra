//! Phase 0 scaffold tests.
//!
//! Verifies that the core math types, unit conversions, and SDF operations
//! compile and produce correct results before any primitives are added.

use approx::assert_abs_diff_eq;

// ── Units ─────────────────────────────────────────────────────────────────

#[test]
fn unit_mm_identity() {
    assert_abs_diff_eq!(_core::units::to_mm(10.0, "mm"), 10.0, epsilon = 1e-4);
}

#[test]
fn unit_cm_to_mm() {
    assert_abs_diff_eq!(_core::units::to_mm(1.0, "cm"), 10.0, epsilon = 1e-4);
}

#[test]
fn unit_m_to_mm() {
    assert_abs_diff_eq!(_core::units::to_mm(1.0, "m"), 1000.0, epsilon = 1e-3);
}

#[test]
fn unit_inch_to_mm() {
    assert_abs_diff_eq!(_core::units::to_mm(1.0, "in"), 25.4, epsilon = 1e-4);
}

#[test]
fn unit_ft_to_mm() {
    assert_abs_diff_eq!(_core::units::to_mm(1.0, "ft"), 304.8, epsilon = 1e-3);
}

#[test]
fn unit_round_trip_cm() {
    let mm = _core::units::to_mm(5.0, "cm");
    assert_abs_diff_eq!(_core::units::from_mm(mm, "cm"), 5.0, epsilon = 1e-4);
}

// ── Arithmetic ────────────────────────────────────────────────────────────

#[test]
fn arithmetic_twice() {
    assert_abs_diff_eq!(
        _core::units::apply_arithmetic(10.0, "twice", 0.0),
        20.0,
        epsilon = 1e-4
    );
}

#[test]
fn arithmetic_half() {
    assert_abs_diff_eq!(
        _core::units::apply_arithmetic(10.0, "half", 0.0),
        5.0,
        epsilon = 1e-4
    );
}

// ── SDF operations ────────────────────────────────────────────────────────

#[test]
fn sdf_union_picks_minimum() {
    use _core::sdf::operations::union;
    assert_abs_diff_eq!(union(3.0, 5.0), 3.0, epsilon = 1e-6);
    assert_abs_diff_eq!(union(-1.0, 2.0), -1.0, epsilon = 1e-6);
}

#[test]
fn sdf_difference_cuts_b_from_a() {
    use _core::sdf::operations::difference;
    // Inside A (a=-1), inside B (b=-2) → the point IS inside B, so cut → positive
    let a = -1.0_f32;
    let b = -2.0_f32;
    assert!(
        difference(a, b) > 0.0,
        "point inside both should be cut out"
    );
}

#[test]
fn sdf_intersection_picks_maximum() {
    use _core::sdf::operations::intersection;
    assert_abs_diff_eq!(intersection(3.0, 5.0), 5.0, epsilon = 1e-6);
}

#[test]
fn sdf_smooth_min_at_k_zero_equals_union() {
    use _core::sdf::operations::{smooth_min, union};
    let a = 2.0_f32;
    let b = 4.0_f32;
    assert_abs_diff_eq!(smooth_min(a, b, 0.0), union(a, b), epsilon = 1e-5);
}

#[test]
fn sdf_smooth_min_blends_at_surface() {
    use _core::sdf::operations::smooth_min;
    // At the surface junction (both = 0), smooth_min dips below 0 — the fillet.
    let result = smooth_min(0.0, 0.0, 4.0);
    assert!(
        result < 0.0,
        "smooth_min at equal-zero surfaces should give negative (fillet dip)"
    );
}

#[test]
fn sdf_smooth_min_matches_union_far_from_boundary() {
    use _core::sdf::operations::{smooth_min, union};
    // Far from the blend zone, smooth_min must equal union.
    let a = 100.0_f32;
    let b = 1.0_f32;
    assert_abs_diff_eq!(smooth_min(a, b, 2.0), union(a, b), epsilon = 1e-3);
}

// ── Math types ────────────────────────────────────────────────────────────

#[test]
fn transform_identity_is_no_op() {
    use _core::math::{Transform, Vec3};
    let t = Transform::identity();
    let p = Vec3::new(1.0, 2.0, 3.0);
    let result = t.transform_point(p);
    assert_abs_diff_eq!(result.x, p.x, epsilon = 1e-5);
    assert_abs_diff_eq!(result.y, p.y, epsilon = 1e-5);
    assert_abs_diff_eq!(result.z, p.z, epsilon = 1e-5);
}

#[test]
fn transform_translate_roundtrip() {
    use _core::math::{Transform, Vec3};
    let offset = Vec3::new(5.0, -3.0, 10.0);
    let t = Transform::from_translation(offset);
    let p = Vec3::new(1.0, 1.0, 1.0);
    let moved = t.transform_point(p);
    let restored = t.inverse_transform_point(moved);
    assert_abs_diff_eq!(restored.x, p.x, epsilon = 1e-4);
    assert_abs_diff_eq!(restored.y, p.y, epsilon = 1e-4);
    assert_abs_diff_eq!(restored.z, p.z, epsilon = 1e-4);
}

#[test]
fn ray_at_parameter() {
    use _core::math::{Ray, Vec3};
    let ray = Ray::new(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
    let point = ray.at(5.0);
    assert_abs_diff_eq!(point.x, 5.0, epsilon = 1e-5);
    assert_abs_diff_eq!(point.y, 0.0, epsilon = 1e-5);
}
