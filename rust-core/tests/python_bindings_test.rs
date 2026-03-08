//! Compile-time and Rust-level checks for the Python bindings layer.
//!
//! Full Python-interpreter tests run via:
//!   `maturin develop --features python && python -m pytest tests/`
//!
//! This file verifies the binding *logic* without the PyO3 runtime by
//! exercising the same Rust types that `src/python.rs` wraps.

use _core::export::{ExportFormat, to_bytes};
use _core::manipulations::{OffsetNode, ShellNode};
use _core::mesher::{MeshConfig, mesh};
use _core::sdf::Sdf;
use _core::sdf::operations::{DifferenceNode, IntersectionNode, SmoothUnionNode, UnionNode};
use _core::sdf::primitives::{CubeSdf, CylinderSdf, PrismSdf, SphereSdf, TorusSdf};
use _core::sdf::transform::{Mirror, MirrorPlane, Rotate, Scale, Translate};
use glam::Vec3;

// ── Distance checks (mirrors PySdfNode.distance()) ────────────────────────────

#[test]
fn cube_inside_at_origin() {
    let c = CubeSdf::new(10.0, 10.0, 10.0);
    assert!(c.distance(Vec3::ZERO) < 0.0);
}

#[test]
fn sphere_inside_at_origin() {
    let s = SphereSdf::new(5.0);
    assert!(s.distance(Vec3::ZERO) < 0.0);
}

#[test]
fn sphere_outside_far_point() {
    let s = SphereSdf::new(5.0);
    assert!(s.distance(Vec3::new(20.0, 0.0, 0.0)) > 0.0);
}

// ── Boolean operations (mirrors PySdfNode.union() etc.) ───────────────────────

#[test]
fn union_inside_at_origin() {
    let u = UnionNode {
        a: Box::new(CubeSdf::new(10.0, 10.0, 10.0)),
        b: Box::new(SphereSdf::new(5.0)),
    };
    assert!(u.distance(Vec3::ZERO) < 0.0);
}

#[test]
fn difference_removes_sphere_core() {
    let d = DifferenceNode {
        a: Box::new(CubeSdf::new(10.0, 10.0, 10.0)),
        b: Box::new(SphereSdf::new(5.0)),
    };
    // Origin is inside the sphere → removed from A → outside the difference
    assert!(d.distance(Vec3::ZERO) > 0.0);
}

#[test]
fn intersection_inside_at_origin() {
    let i = IntersectionNode {
        a: Box::new(CubeSdf::new(10.0, 10.0, 10.0)),
        b: Box::new(SphereSdf::new(5.0)),
    };
    assert!(i.distance(Vec3::ZERO) < 0.0);
}

#[test]
fn smooth_union_blends_between_spheres() {
    let su = SmoothUnionNode {
        a: Box::new(SphereSdf::new(5.0)),
        b: Box::new(Translate {
            inner: Box::new(SphereSdf::new(5.0)),
            offset: Vec3::new(6.0, 0.0, 0.0),
        }),
        k: 2.0,
    };
    // Midpoint should be inside due to blending
    assert!(su.distance(Vec3::new(3.0, 0.0, 0.0)) < 0.0);
}

// ── Transforms (mirrors PySdfNode.translate(), .scale(), etc.) ────────────────

#[test]
fn translate_moves_sphere() {
    let t = Translate {
        inner: Box::new(SphereSdf::new(5.0)),
        offset: Vec3::new(20.0, 0.0, 0.0),
    };
    assert!(t.distance(Vec3::ZERO) > 0.0);
    assert!(t.distance(Vec3::new(20.0, 0.0, 0.0)) < 0.0);
}

#[test]
fn scale_enlarges_sphere() {
    let s = Scale {
        inner: Box::new(SphereSdf::new(5.0)),
        factor: 3.0,
    };
    assert!(s.distance(Vec3::new(12.0, 0.0, 0.0)) < 0.0);
}

#[test]
fn mirror_yz_reflects_sphere() {
    let t = Translate {
        inner: Box::new(SphereSdf::new(3.0)),
        offset: Vec3::new(8.0, 0.0, 0.0),
    };
    let m = Mirror {
        inner: Box::new(t),
        plane: MirrorPlane::Yz,
    };
    // Both +8 and -8 should be inside
    assert!(m.distance(Vec3::new(8.0, 0.0, 0.0)) < 0.0);
    assert!(m.distance(Vec3::new(-8.0, 0.0, 0.0)) < 0.0);
}

#[test]
fn rotate_z_90_maps_x_to_y() {
    // Cube is 10 wide in local X, 2 wide in local Y.
    // After a 90° rotation around Z, local X → world Y, local Y → world -X.
    // The Rotate node applies the INVERSE rotation to the query point.
    // Query (world X = 0.8, Y = 0) → inverse(-90°) → local (x' = 0, y' = -0.8).
    //   Local Y = -0.8, inside ±1 slab → inside.
    // Query (world X = 3.0, Y = 0) → inverse(-90°) → local (x' = 0, y' = -3.0).
    //   Local Y = -3.0, outside ±1 slab → outside.
    let r = Rotate::new(
        Box::new(CubeSdf::new(10.0, 2.0, 10.0)), // narrow (2) along local Y
        glam::Quat::from_rotation_z(90f32.to_radians()),
    );
    assert!(
        r.distance(Vec3::new(0.8, 0.0, 0.0)) < 0.0,
        "inside narrow dim"
    );
    assert!(
        r.distance(Vec3::new(3.0, 0.0, 0.0)) > 0.0,
        "outside narrow dim"
    );
}

// ── Manipulations ─────────────────────────────────────────────────────────────

#[test]
fn shell_hollows_cube() {
    let s = ShellNode {
        inner: Box::new(CubeSdf::new(10.0, 10.0, 10.0)),
        thickness: 1.5,
    };
    assert!(s.distance(Vec3::ZERO) > 0.0); // deep inside → hollow
    assert!(s.distance(Vec3::new(4.5, 0.0, 0.0)) < 0.0); // near wall → inside shell
}

#[test]
fn offset_grows_sphere() {
    let o = OffsetNode {
        inner: Box::new(SphereSdf::new(5.0)),
        offset: 3.0,
    };
    assert!(o.distance(Vec3::new(7.0, 0.0, 0.0)) < 0.0);
    assert!(o.distance(Vec3::new(10.0, 0.0, 0.0)) > 0.0);
}

// ── Meshing + export (mirrors mesh_sdf() + PyMesh.to_*()) ─────────────────────

#[test]
fn mesh_and_export_all_formats() {
    let cfg = MeshConfig::centered(12.0, 16);
    let sdf = CubeSdf::new(10.0, 10.0, 10.0);
    let m = mesh(&sdf, &cfg);
    assert!(m.triangle_count() > 0);

    for fmt in [
        ExportFormat::Stl,
        ExportFormat::Obj,
        ExportFormat::Ply,
        ExportFormat::Glb,
    ] {
        let bytes = to_bytes(&m, fmt).unwrap();
        assert!(!bytes.is_empty(), "{fmt:?} must produce output");
    }
}

#[test]
fn mesh_sphere_has_reasonable_triangle_count() {
    let cfg = MeshConfig::centered(10.0, 20);
    let sdf = SphereSdf::new(7.0);
    let m = mesh(&sdf, &cfg);
    assert!(m.triangle_count() > 50, "sphere mesh too coarse");
}

// ── All primitive constructors reachable ──────────────────────────────────────

#[test]
fn all_solid_primitives_inside_at_origin() {
    // Solid primitives (centre-at-origin) are inside at Vec3::ZERO
    let primitives: Vec<Box<dyn Sdf>> = vec![
        Box::new(CubeSdf::new(10.0, 10.0, 10.0)),
        Box::new(SphereSdf::new(5.0)),
        Box::new(CylinderSdf::new(4.0, 10.0)),
        Box::new(PrismSdf::new(6, 8.0, 10.0)),
    ];
    for p in &primitives {
        assert!(
            p.distance(Vec3::ZERO) < 0.0,
            "primitive must be inside at origin"
        );
    }
}

#[test]
fn torus_inside_at_tube_centre() {
    // Torus: major=6, minor=2. Inside is the tube, not the hole.
    // The tube centre nearest to +X axis is at (6, 0, 0).
    let t = TorusSdf::new(6.0, 2.0);
    assert!(t.distance(Vec3::new(6.0, 0.0, 0.0)) < 0.0);
    assert!(t.distance(Vec3::ZERO) > 0.0, "centre hole is outside");
}

#[test]
fn cone_and_pyramid_construct() {
    use _core::sdf::primitives::{ConeSdf, PyramidSdf};
    // Just verify they construct and return plausible distances
    let cone = ConeSdf::new(4.0, 0.0, 10.0);
    let pyr = PyramidSdf::new(8.0, 8.0, 10.0);
    // Query well outside both — should be positive
    assert!(cone.distance(Vec3::new(100.0, 0.0, 0.0)) > 0.0);
    assert!(pyr.distance(Vec3::new(100.0, 0.0, 0.0)) > 0.0);
}
