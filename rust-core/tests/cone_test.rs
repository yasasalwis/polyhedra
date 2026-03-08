//! Integration tests for ConeSdf.

use approx::assert_abs_diff_eq;
use _core::sdf::primitives::ConeSdf;
use _core::sdf::Sdf;
use glam::Vec3;

fn pointed() -> ConeSdf { ConeSdf::new(5.0, 0.0, 10.0) }
fn frustum() -> ConeSdf { ConeSdf::new(6.0, 3.0, 10.0) }

// ── Interior ──────────────────────────────────────────────────────────────

#[test]
fn pointed_center_inside() {
    assert!(pointed().distance(Vec3::ZERO) < 0.0);
}

#[test]
fn frustum_center_inside() {
    assert!(frustum().distance(Vec3::ZERO) < 0.0);
}

// ── Surface ───────────────────────────────────────────────────────────────

#[test]
fn pointed_tip_on_surface() {
    let d = pointed().distance(Vec3::new(0.0, 0.0, 5.0));
    assert!(d.abs() < 1e-4, "pointed tip: {d}");
}

#[test]
fn pointed_base_center_on_surface() {
    let d = pointed().distance(Vec3::new(0.0, 0.0, -5.0));
    assert!(d.abs() < 1e-4, "pointed base: {d}");
}

#[test]
fn pointed_base_rim_on_surface() {
    let d = pointed().distance(Vec3::new(5.0, 0.0, -5.0));
    assert!(d.abs() < 1e-4, "pointed base rim: {d}");
}

#[test]
fn frustum_base_rim_on_surface() {
    let d = frustum().distance(Vec3::new(6.0, 0.0, -5.0));
    assert!(d.abs() < 1e-4, "frustum base rim: {d}");
}

#[test]
fn frustum_top_rim_on_surface() {
    let d = frustum().distance(Vec3::new(3.0, 0.0, 5.0));
    assert!(d.abs() < 1e-4, "frustum top rim: {d}");
}

// ── Exterior ─────────────────────────────────────────────────────────────

#[test]
fn pointed_above_tip() {
    assert_abs_diff_eq!(pointed().distance(Vec3::new(0.0, 0.0, 8.0)), 3.0, epsilon = 1e-4);
}

#[test]
fn pointed_below_base() {
    assert_abs_diff_eq!(pointed().distance(Vec3::new(0.0, 0.0, -9.0)), 4.0, epsilon = 1e-4);
}

#[test]
fn frustum_above_top() {
    assert_abs_diff_eq!(frustum().distance(Vec3::new(0.0, 0.0, 8.0)), 3.0, epsilon = 1e-4);
}

#[test]
fn frustum_below_base() {
    assert_abs_diff_eq!(frustum().distance(Vec3::new(0.0, 0.0, -8.0)), 3.0, epsilon = 1e-4);
}

// ── Shape-type mapping ────────────────────────────────────────────────────

#[test]
fn shape_type_cone_id_is_three() {
    use _core::shape_type::ShapeType;
    assert_eq!(ShapeType::try_from(3u32).unwrap(), ShapeType::Cone);
}

// ── Trait object ──────────────────────────────────────────────────────────

#[test]
fn cone_as_sdf_node() {
    use _core::sdf::SdfNode;
    let node: SdfNode = Box::new(ConeSdf::new(5.0, 0.0, 10.0));
    assert!(node.distance(Vec3::ZERO) < 0.0);
    assert!(node.distance(Vec3::new(20.0, 0.0, 0.0)) > 0.0);
}

// ── Cut cone from cylinder ─────────────────────────────────────────────────

#[test]
fn cone_cut_from_cylinder() {
    use _core::sdf::operations::DifferenceNode;
    use _core::sdf::primitives::CylinderSdf;
    use _core::sdf::SdfNode;

    // Cylinder r=8, h=20, with a cone (base=6, top=0, h=22) drilled from top.
    let cyl: SdfNode  = Box::new(CylinderSdf::new(8.0, 20.0));
    let cone: SdfNode = Box::new(ConeSdf::new(6.0, 0.0, 22.0));
    let result: SdfNode = Box::new(DifferenceNode { a: cyl, b: cone });

    // On the Z axis inside the cone cavity → outside the result.
    assert!(result.distance(Vec3::new(0.0, 0.0, 0.0)) > 0.0);
    // Off-axis inside the cylinder, outside the cone → inside the result.
    assert!(result.distance(Vec3::new(7.0, 0.0, 0.0)) < 0.0);
}
