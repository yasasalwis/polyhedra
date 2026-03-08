//! Integration tests for CylinderSdf.

use _core::sdf::Sdf;
use _core::sdf::primitives::CylinderSdf;
use approx::assert_abs_diff_eq;
use glam::Vec3;

/// r=5, h=20.
fn cyl() -> CylinderSdf {
    CylinderSdf::new(5.0, 20.0)
}

// ── Interior ──────────────────────────────────────────────────────────────

#[test]
fn cylinder_center_inside() {
    assert!(cyl().distance(Vec3::ZERO) < 0.0);
}

#[test]
fn cylinder_center_distance() {
    // min(r=5, half_h=10) = 5 → d = -5.
    assert_abs_diff_eq!(cyl().distance(Vec3::ZERO), -5.0, epsilon = 1e-5);
}

// ── Surface ───────────────────────────────────────────────────────────────

#[test]
fn curved_surface_is_zero() {
    let c = cyl();
    for z in [-9.0_f32, 0.0, 9.0] {
        let d = c.distance(Vec3::new(5.0, 0.0, z));
        assert!(d.abs() < 1e-5, "z={z}: expected ~0, got {d}");
    }
}

#[test]
fn top_cap_is_zero() {
    assert_abs_diff_eq!(
        cyl().distance(Vec3::new(0.0, 0.0, 10.0)),
        0.0,
        epsilon = 1e-5
    );
}

#[test]
fn bottom_cap_is_zero() {
    assert_abs_diff_eq!(
        cyl().distance(Vec3::new(0.0, 0.0, -10.0)),
        0.0,
        epsilon = 1e-5
    );
}

#[test]
fn rim_is_on_surface() {
    let c = cyl();
    assert_abs_diff_eq!(c.distance(Vec3::new(5.0, 0.0, 10.0)), 0.0, epsilon = 1e-5);
    assert_abs_diff_eq!(c.distance(Vec3::new(5.0, 0.0, -10.0)), 0.0, epsilon = 1e-5);
}

// ── Exterior ─────────────────────────────────────────────────────────────

#[test]
fn outside_radially() {
    assert_abs_diff_eq!(
        cyl().distance(Vec3::new(8.0, 0.0, 0.0)),
        3.0,
        epsilon = 1e-5
    );
}

#[test]
fn outside_axially() {
    assert_abs_diff_eq!(
        cyl().distance(Vec3::new(0.0, 0.0, 14.0)),
        4.0,
        epsilon = 1e-5
    );
}

#[test]
fn outside_corner_distance() {
    // 3 mm past radius, 4 mm above cap → distance to rim edge = sqrt(9+16) = 5.
    assert_abs_diff_eq!(
        cyl().distance(Vec3::new(8.0, 0.0, 14.0)),
        5.0,
        epsilon = 1e-4
    );
}

// ── Shape-type mapping ────────────────────────────────────────────────────

#[test]
fn shape_type_cylinder_id_is_one() {
    use _core::shape_type::ShapeType;
    assert_eq!(ShapeType::try_from(1u32).unwrap(), ShapeType::Cylinder);
}

// ── Trait object ──────────────────────────────────────────────────────────

#[test]
fn cylinder_as_sdf_node() {
    use _core::sdf::SdfNode;
    let node: SdfNode = Box::new(CylinderSdf::new(5.0, 20.0));
    assert!(node.distance(Vec3::ZERO) < 0.0);
    assert!(node.distance(Vec3::new(20.0, 0.0, 0.0)) > 0.0);
}

// ── Difference: cylinder cut from cube ───────────────────────────────────

#[test]
fn cylinder_cut_from_cube() {
    use _core::sdf::SdfNode;
    use _core::sdf::operations::DifferenceNode;
    use _core::sdf::primitives::CubeSdf;

    // 20×20×20 cube with a r=3, h=25 hole drilled through it along Z.
    let cube: SdfNode = Box::new(CubeSdf::new(20.0, 20.0, 20.0));
    let hole: SdfNode = Box::new(CylinderSdf::new(3.0, 25.0));
    let result: SdfNode = Box::new(DifferenceNode { a: cube, b: hole });

    // On the Z axis (inside the hole) → outside the result.
    assert!(result.distance(Vec3::new(0.0, 0.0, 0.0)) > 0.0);
    // Off-axis, inside the cube but outside the hole → inside the result.
    assert!(result.distance(Vec3::new(8.0, 0.0, 0.0)) < 0.0);
    // Completely outside the cube → outside the result.
    assert!(result.distance(Vec3::new(30.0, 0.0, 0.0)) > 0.0);
}
