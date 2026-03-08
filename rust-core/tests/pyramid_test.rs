//! Integration tests for PyramidSdf.

use _core::sdf::Sdf;
use _core::sdf::primitives::PyramidSdf;
use approx::assert_abs_diff_eq;
use glam::Vec3;

fn square_pyr() -> PyramidSdf {
    PyramidSdf::new(10.0, 10.0, 10.0)
}
fn rect_pyr() -> PyramidSdf {
    PyramidSdf::new(20.0, 10.0, 10.0)
}

// ── Surface ───────────────────────────────────────────────────────────────

#[test]
fn apex_on_surface() {
    let d = square_pyr().distance(Vec3::new(0.0, 0.0, 5.0));
    assert!(d.abs() < 1e-5, "apex: {d}");
}

#[test]
fn base_center_on_surface() {
    let d = square_pyr().distance(Vec3::new(0.0, 0.0, -5.0));
    assert!(d.abs() < 1e-5, "base centre: {d}");
}

#[test]
fn base_corner_on_surface() {
    let d = square_pyr().distance(Vec3::new(5.0, 5.0, -5.0));
    assert!(d.abs() < 1e-5, "base corner: {d}");
}

#[test]
fn base_edge_midpoint_on_surface() {
    let d = square_pyr().distance(Vec3::new(5.0, 0.0, -5.0));
    assert!(d.abs() < 1e-5, "base edge mid: {d}");
}

// ── Interior ──────────────────────────────────────────────────────────────

#[test]
fn center_inside() {
    assert!(square_pyr().distance(Vec3::ZERO) < 0.0);
}

// ── Exterior face regions ─────────────────────────────────────────────────

#[test]
fn below_base_on_axis() {
    assert_abs_diff_eq!(
        square_pyr().distance(Vec3::new(0.0, 0.0, -9.0)),
        4.0,
        epsilon = 1e-4,
    );
}

#[test]
fn outside_x_face_is_positive() {
    assert!(square_pyr().distance(Vec3::new(8.0, 0.0, 0.0)) > 0.0);
}

#[test]
fn outside_y_face_is_positive() {
    assert!(square_pyr().distance(Vec3::new(0.0, 8.0, 0.0)) > 0.0);
}

// ── Rectangular pyramid ────────────────────────────────────────────────────

#[test]
fn rect_apex_on_surface() {
    let d = rect_pyr().distance(Vec3::new(0.0, 0.0, 5.0));
    assert!(d.abs() < 1e-5, "rect apex: {d}");
}

#[test]
fn rect_base_corner_on_surface() {
    let d = rect_pyr().distance(Vec3::new(10.0, 5.0, -5.0));
    assert!(d.abs() < 1e-5, "rect base corner: {d}");
}

#[test]
fn rect_below_base() {
    assert_abs_diff_eq!(
        rect_pyr().distance(Vec3::new(0.0, 0.0, -8.0)),
        3.0,
        epsilon = 1e-4,
    );
}

// ── Shape-type mapping ────────────────────────────────────────────────────

#[test]
fn shape_type_pyramid_id_is_five() {
    use _core::shape_type::ShapeType;
    assert_eq!(ShapeType::try_from(5u32).unwrap(), ShapeType::Pyramid);
}

// ── Trait object ──────────────────────────────────────────────────────────

#[test]
fn pyramid_as_sdf_node() {
    use _core::sdf::SdfNode;
    let node: SdfNode = Box::new(PyramidSdf::new(10.0, 10.0, 10.0));
    assert!(node.distance(Vec3::ZERO) < 0.0);
    assert!(node.distance(Vec3::new(0.0, 0.0, 30.0)) > 0.0);
}

// ── Cut pyramid from cube ─────────────────────────────────────────────────

#[test]
fn pyramid_cut_from_cube_gives_void() {
    use _core::sdf::SdfNode;
    use _core::sdf::operations::DifferenceNode;
    use _core::sdf::primitives::CubeSdf;

    // 30×30×30 cube with a pyramid-shaped recess cut into the top.
    let cube: SdfNode = Box::new(CubeSdf::new(30.0, 30.0, 30.0));
    let pyr: SdfNode = Box::new(PyramidSdf::new(10.0, 10.0, 20.0));
    let result: SdfNode = Box::new(DifferenceNode { a: cube, b: pyr });

    // At origin, inside both → cut out → outside result.
    assert!(result.distance(Vec3::ZERO) > 0.0);
    // Off-centre inside the cube, outside the pyramid → inside result.
    assert!(result.distance(Vec3::new(12.0, 0.0, 0.0)) < 0.0);
    // Completely outside → outside result.
    assert!(result.distance(Vec3::new(50.0, 0.0, 0.0)) > 0.0);
}
