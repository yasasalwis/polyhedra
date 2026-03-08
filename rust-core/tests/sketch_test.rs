//! Integration tests for the Sketch system.

use approx::assert_abs_diff_eq;
use glam::Vec3;

use _core::mesher::{MeshConfig, mesh};
use _core::sdf::{
    Sdf,
    primitives::{CubeSdf, CylinderSdf, PrismSdf, SphereSdf, TorusSdf},
};
use _core::sketch::{
    ops::{ExtrudeNode, RevolveAxis, RevolveNode},
    primitives::{Capsule, Circle, Rect, RegularPolygon, RoundedRect},
    {Difference2d, Intersection2d, Offset2d, Sdf2d, Sdf2dNode, Union2d},
};

// ── 2D primitives ─────────────────────────────────────────────────────────────

#[test]
fn circle_surface_on_all_quadrants() {
    let c = Circle::new(3.0);
    for &(x, y) in &[(3.0, 0.0), (-3.0, 0.0), (0.0, 3.0), (0.0, -3.0)] {
        assert_abs_diff_eq!(c.distance(glam::Vec2::new(x, y)), 0.0, epsilon = 1e-5);
    }
}

#[test]
fn rect_symmetry() {
    let r = Rect::new(8.0, 4.0);
    let p = glam::Vec2::new(3.0, 1.5);
    // All four quadrant images should give the same distance.
    assert_abs_diff_eq!(
        r.distance(p),
        r.distance(glam::Vec2::new(-3.0, 1.5)),
        epsilon = 1e-5
    );
    assert_abs_diff_eq!(
        r.distance(p),
        r.distance(glam::Vec2::new(3.0, -1.5)),
        epsilon = 1e-5
    );
}

#[test]
fn rounded_rect_inside_rect() {
    // Rounded version always has smaller or equal distance magnitude.
    let r = Rect::new(10.0, 6.0);
    let rr = RoundedRect::new(10.0, 6.0, 1.0);
    // At face centre: both zero.
    assert_abs_diff_eq!(r.distance(glam::Vec2::new(5.0, 0.0)), 0.0, epsilon = 1e-5);
    assert_abs_diff_eq!(rr.distance(glam::Vec2::new(5.0, 0.0)), 0.0, epsilon = 1e-5);
    // At sharp corner: rounded is positive (outside), rect is zero.
    assert!(rr.distance(glam::Vec2::new(5.0, 3.0)) > 0.0);
    assert_abs_diff_eq!(r.distance(glam::Vec2::new(5.0, 3.0)), 0.0, epsilon = 1e-5);
}

#[test]
fn capsule_length_matches_endpoints() {
    let c = Capsule {
        a: glam::Vec2::new(-4.0, 0.0),
        b: glam::Vec2::new(4.0, 0.0),
        radius: 1.5,
    };
    // End caps at x = ±4 ± 1.5 = ±5.5.
    assert_abs_diff_eq!(c.distance(glam::Vec2::new(5.5, 0.0)), 0.0, epsilon = 1e-5);
    assert_abs_diff_eq!(c.distance(glam::Vec2::new(-5.5, 0.0)), 0.0, epsilon = 1e-5);
}

#[test]
fn regular_hex_matches_prism_2d() {
    // RegularPolygon(6, 5) should give the same 2D distances as the PrismSdf
    // in the z=0 plane.
    let poly = RegularPolygon::new(6, 5.0);
    let prism = PrismSdf::new(6, 10.0, 20.0);

    for &(x, y) in &[(0.0_f32, 0.0), (5.0, 0.0), (8.0, 0.0), (3.0, 4.0)] {
        let p2d = glam::Vec2::new(x, y);
        let p3d = Vec3::new(x, y, 0.0);
        // In the z=0 mid-plane, prism SDF = 2D polygon SDF.
        assert_abs_diff_eq!(poly.distance(p2d), prism.distance(p3d), epsilon = 1e-4,);
    }
}

// ── 2D Boolean operations ──────────────────────────────────────────────────────

#[test]
fn union2d_covers_both() {
    let a: Sdf2dNode = Box::new(Circle::new(3.0));
    let b: Sdf2dNode = Box::new(Rect::new(2.0, 10.0));
    let u = Union2d { a, b };
    assert!(u.distance(glam::Vec2::new(2.5, 0.0)) < 0.0); // inside circle
    assert!(u.distance(glam::Vec2::new(0.0, 4.5)) < 0.0); // inside rect
}

#[test]
fn difference2d_cuts_circle_from_rect() {
    let a: Sdf2dNode = Box::new(Rect::new(10.0, 10.0));
    let b: Sdf2dNode = Box::new(Circle::new(3.0));
    let d = Difference2d { a, b };
    assert!(d.distance(glam::Vec2::ZERO) > 0.0); // hole at centre
    assert!(d.distance(glam::Vec2::new(4.5, 4.5)) < 0.0); // corner far from hole
}

#[test]
fn intersection2d_only_overlap() {
    let a: Sdf2dNode = Box::new(Circle::new(5.0));
    let b: Sdf2dNode = Box::new(Rect::new(4.0, 4.0));
    let i = Intersection2d { a, b };
    assert!(i.distance(glam::Vec2::new(1.5, 1.5)) < 0.0); // inside both
    assert!(i.distance(glam::Vec2::new(4.5, 0.0)) > 0.0); // inside circle, outside rect
}

#[test]
fn offset2d_expands_circle() {
    let inner: Sdf2dNode = Box::new(Circle::new(3.0));
    let expanded = Offset2d { inner, offset: 2.0 };
    assert_abs_diff_eq!(
        expanded.distance(glam::Vec2::new(5.0, 0.0)),
        0.0,
        epsilon = 1e-5
    );
}

// ── ExtrudeNode ───────────────────────────────────────────────────────────────

#[test]
fn extrude_circle_surface_matches_cylinder() {
    let extrude = ExtrudeNode::new(Box::new(Circle::new(4.0)), 10.0);
    let cyl = CylinderSdf::new(4.0, 10.0);

    for p in [
        Vec3::ZERO,
        Vec3::new(4.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(4.0, 0.0, 5.0),
        Vec3::new(6.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 8.0),
    ] {
        assert_abs_diff_eq!(extrude.distance(p), cyl.distance(p), epsilon = 1e-4);
    }
}

#[test]
fn extrude_rect_matches_cube() {
    let extrude = ExtrudeNode::new(Box::new(Rect::new(10.0, 6.0)), 4.0);
    let cube = CubeSdf::new(10.0, 6.0, 4.0);

    for p in [
        Vec3::ZERO,
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(0.0, 3.0, 0.0),
        Vec3::new(0.0, 0.0, 2.0),
    ] {
        assert_abs_diff_eq!(extrude.distance(p), cube.distance(p), epsilon = 1e-4);
    }
}

#[test]
fn extrude_rounded_rect_is_smooth() {
    // Extrude a rounded rectangle and mesh it — should produce a valid mesh.
    let profile: _core::sketch::Sdf2dNode = Box::new(RoundedRect::new(8.0, 6.0, 1.0));
    let extrude = ExtrudeNode::new(profile, 4.0);
    let m = mesh(&extrude, &MeshConfig::centered(7.0, 24));
    assert!(m.vertex_count() > 0);
    assert!(m.triangle_count() > 12);
    assert!(m.is_index_valid());
}

#[test]
fn extrude_capsule_profile() {
    // Extruded capsule: stadium-shaped prism.
    let cap = Capsule::horizontal(3.0, 2.0);
    let extrude = ExtrudeNode::new(Box::new(cap), 6.0);
    // On axis: inside.
    assert!(extrude.distance(Vec3::ZERO) < 0.0);
    // Beyond end cap: outside.
    assert!(extrude.distance(Vec3::new(6.0, 0.0, 0.0)) > 0.0);
}

#[test]
fn extrude_hexagon_matches_prism() {
    let extrude = ExtrudeNode::new(Box::new(RegularPolygon::new(6, 5.0)), 20.0);
    let prism = PrismSdf::new(6, 10.0, 20.0);

    for p in [
        Vec3::ZERO,
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::new(8.0, 0.0, 5.0),
    ] {
        assert_abs_diff_eq!(extrude.distance(p), prism.distance(p), epsilon = 1e-4);
    }
}

// ── RevolveNode ───────────────────────────────────────────────────────────────

#[test]
fn revolve_circle_z_matches_torus() {
    let major = 6.0_f32;
    let minor = 1.5_f32;
    let revolve = RevolveNode::around_z(Box::new(Circle::new(minor)), major);
    let torus = TorusSdf::new(major, minor);

    for p in [
        Vec3::new(major + minor, 0.0, 0.0),
        Vec3::new(major - minor, 0.0, 0.0),
        Vec3::new(0.0, major + minor, 0.0),
        Vec3::new(major, 0.0, minor),
        Vec3::ZERO,
    ] {
        assert_abs_diff_eq!(revolve.distance(p), torus.distance(p), epsilon = 1e-4);
    }
}

#[test]
fn revolve_rect_produces_annular_disk() {
    // Revolve a thin rect (2 wide, 1 tall) at offset 5 → flat ring.
    let revolve = RevolveNode::around_z(Box::new(Rect::new(2.0, 1.0)), 5.0);
    // Centre of profile orbit at (5,0,0): inside.
    assert!(revolve.distance(Vec3::new(5.0, 0.0, 0.0)) < 0.0);
    // Axis (centre): outside (hole).
    assert!(revolve.distance(Vec3::ZERO) > 0.0);
    // Top of the ring at z=0.5: on surface.
    assert_abs_diff_eq!(
        revolve.distance(Vec3::new(5.0, 0.0, 0.5)),
        0.0,
        epsilon = 1e-4
    );
}

#[test]
fn revolve_zero_offset_gives_sphere_metric() {
    // Circle(r) revolved around Z with offset=0 → sphere.
    let r = 4.0_f32;
    let revolve = RevolveNode::around_z(Box::new(Circle::new(r)), 0.0);
    let sphere = SphereSdf::new(r);

    for p in [
        Vec3::new(r, 0.0, 0.0),
        Vec3::new(0.0, r, 0.0),
        Vec3::new(0.0, 0.0, r),
        Vec3::ZERO,
    ] {
        assert_abs_diff_eq!(revolve.distance(p), sphere.distance(p), epsilon = 1e-4);
    }
}

#[test]
fn revolve_axis_x() {
    // Circle(r=2) around X, offset=5 → torus around X axis.
    // Surface in YZ plane at sqrt(y²+z²) = 5+2 = 7.
    let revolve = RevolveNode {
        profile: Box::new(Circle::new(2.0)),
        axis: RevolveAxis::X,
        offset: 5.0,
    };
    // On Y axis at (0, 7, 0): u = sqrt(49) - 5 = 2, q=(2,0), d=0. ✓
    assert_abs_diff_eq!(
        revolve.distance(Vec3::new(0.0, 7.0, 0.0)),
        0.0,
        epsilon = 1e-4
    );
}

// ── Sketch → CSG compositions ──────────────────────────────────────────────────

#[test]
fn extruded_donut_cross_section() {
    // Extrude a "donut" 2D profile: circle minus inner circle.
    let outer: Sdf2dNode = Box::new(Circle::new(5.0));
    let inner: Sdf2dNode = Box::new(Circle::new(3.0));
    let ring = Difference2d { a: outer, b: inner };
    let extrude = ExtrudeNode::new(Box::new(ring), 4.0);

    // On axis (inside inner hole) → outside.
    assert!(extrude.distance(Vec3::ZERO) > 0.0);
    // In the ring (r=4) → inside.
    assert!(extrude.distance(Vec3::new(4.0, 0.0, 0.0)) < 0.0);
    // Beyond outer edge → outside.
    assert!(extrude.distance(Vec3::new(8.0, 0.0, 0.0)) > 0.0);
}

#[test]
fn sketch_mesh_is_valid() {
    // Complex profile: two overlapping circles.
    let a: Sdf2dNode = Box::new(Circle::new(4.0));
    let profile = Union2d {
        a,
        b: Box::new(Rect::new(3.0, 8.0)),
    };
    let extrude = ExtrudeNode::new(Box::new(profile), 6.0);
    let m = mesh(&extrude, &MeshConfig::centered(7.0, 20));
    assert!(m.vertex_count() > 0);
    assert!(m.triangle_count() > 0);
    assert!(m.is_index_valid());
}
