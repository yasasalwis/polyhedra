//! Integration tests for PrismSdf.

use approx::assert_abs_diff_eq;
use _core::sdf::primitives::PrismSdf;
use _core::sdf::Sdf;
use glam::Vec3;
use std::f32::consts::TAU;

fn hex() -> PrismSdf { PrismSdf::new(6, 10.0, 20.0) }
fn tri() -> PrismSdf { PrismSdf::new(3, 10.0, 20.0) }

// ── Interior ──────────────────────────────────────────────────────────────

#[test]
fn hex_center_inside() {
    assert!(hex().distance(Vec3::ZERO) < 0.0);
}

#[test]
fn hex_center_distance() {
    // apothem=5, nearest face = 5 mm.
    assert_abs_diff_eq!(hex().distance(Vec3::ZERO), -5.0, epsilon = 1e-4);
}

// ── Surface — face centres ────────────────────────────────────────────────

#[test]
fn hex_all_face_centres_on_surface() {
    let h = hex();
    for k in 0..6u32 {
        let angle = k as f32 * TAU / 6.0;
        let p = Vec3::new(5.0 * angle.cos(), 5.0 * angle.sin(), 0.0);
        let d = h.distance(p);
        assert!(d.abs() < 1e-4, "face {k}: {d}");
    }
}

// ── Surface — corner vertices ──────────────────────────────────────────────

#[test]
fn hex_corner_vertices_on_surface() {
    let h = hex();
    let r = h.circumradius();
    for k in 0..6u32 {
        let angle = (k as f32 + 0.5) * TAU / 6.0;
        let p = Vec3::new(r * angle.cos(), r * angle.sin(), 0.0);
        let d = h.distance(p);
        assert!(d.abs() < 1e-4, "vertex {k}: {d}");
    }
}

#[test]
fn tri_vertex_on_surface() {
    let t = tri();
    let r = t.circumradius();
    let d = t.distance(Vec3::new(r * 60_f32.to_radians().cos(), r * 60_f32.to_radians().sin(), 0.0));
    assert!(d.abs() < 1e-4, "tri vertex: {d}");
}

// ── Surface — caps ────────────────────────────────────────────────────────

#[test]
fn hex_top_cap_on_surface() {
    let d = hex().distance(Vec3::new(0.0, 0.0, 10.0));
    assert!(d.abs() < 1e-4, "top: {d}");
}

#[test]
fn hex_bottom_cap_on_surface() {
    let d = hex().distance(Vec3::new(0.0, 0.0, -10.0));
    assert!(d.abs() < 1e-4, "bottom: {d}");
}

// ── Exterior ─────────────────────────────────────────────────────────────

#[test]
fn hex_outside_face_distance() {
    assert_abs_diff_eq!(hex().distance(Vec3::new(8.0, 0.0, 0.0)), 3.0, epsilon = 1e-4);
}

#[test]
fn hex_outside_axially() {
    assert_abs_diff_eq!(hex().distance(Vec3::new(0.0, 0.0, 14.0)), 4.0, epsilon = 1e-4);
}

#[test]
fn hex_outside_both() {
    // 3 radial + 4 axial → sqrt(9+16) = 5.
    assert_abs_diff_eq!(
        hex().distance(Vec3::new(8.0, 0.0, 14.0)),
        5.0,
        epsilon = 1e-3,
    );
}

// ── Shape-type mapping ────────────────────────────────────────────────────

#[test]
fn shape_type_prism_id_is_six() {
    use _core::shape_type::ShapeType;
    assert_eq!(ShapeType::try_from(6u32).unwrap(), ShapeType::Prism);
}

// ── Trait object ──────────────────────────────────────────────────────────

#[test]
fn prism_as_sdf_node() {
    use _core::sdf::SdfNode;
    let node: SdfNode = Box::new(PrismSdf::new(6, 10.0, 20.0));
    assert!(node.distance(Vec3::ZERO) < 0.0);
    assert!(node.distance(Vec3::new(30.0, 0.0, 0.0)) > 0.0);
}

// ── n=4 agrees with apothem convention ────────────────────────────────────

#[test]
fn square_prism_face_distance() {
    let sq = PrismSdf::new(4, 10.0, 10.0);
    // 3 mm past the +X face (apothem=5, so face at x=5, point at x=8).
    assert_abs_diff_eq!(sq.distance(Vec3::new(8.0, 0.0, 0.0)), 3.0, epsilon = 1e-4);
}

// ── Hexagonal prism cut from cube ─────────────────────────────────────────

#[test]
fn hex_prism_cut_from_cube() {
    use _core::sdf::operations::DifferenceNode;
    use _core::sdf::primitives::CubeSdf;
    use _core::sdf::SdfNode;

    // 50×50×50 cube with a hexagonal bore (r=4, h=60) drilled through it.
    let cube: SdfNode = Box::new(CubeSdf::new(50.0, 50.0, 50.0));
    let bore: SdfNode = Box::new(PrismSdf::new(6, 8.0, 60.0));
    let result: SdfNode = Box::new(DifferenceNode { a: cube, b: bore });

    // On axis inside the bore → outside result.
    assert!(result.distance(Vec3::ZERO) > 0.0);
    // Off-axis inside cube, outside bore → inside result.
    assert!(result.distance(Vec3::new(20.0, 0.0, 0.0)) < 0.0);
    // Far outside cube → outside result.
    assert!(result.distance(Vec3::new(100.0, 0.0, 0.0)) > 0.0);
}
