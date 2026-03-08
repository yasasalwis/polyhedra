//! Integration tests for the Dual Contouring mesher.

use _core::mesher::{MeshConfig, mesh};
use _core::sdf::SdfNode;
use _core::sdf::operations::UnionNode;
use _core::sdf::primitives::{CubeSdf, CylinderSdf, SphereSdf};
use glam::Vec3;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Coarse resolution — fast enough for CI.
fn coarse(half: f32) -> MeshConfig {
    MeshConfig::centered(half, 24)
}

// ── Sphere ────────────────────────────────────────────────────────────────────

#[test]
fn sphere_produces_vertices_and_triangles() {
    let m = mesh(&SphereSdf::new(5.0), &coarse(7.0));
    assert!(m.vertex_count() > 0, "no vertices");
    assert!(m.triangle_count() > 0, "no triangles");
}

#[test]
fn sphere_indices_are_valid() {
    let m = mesh(&SphereSdf::new(5.0), &coarse(7.0));
    assert!(m.is_index_valid(), "out-of-range triangle index");
}

#[test]
fn sphere_vertex_count_scales_with_resolution() {
    let lo = mesh(&SphereSdf::new(5.0), &MeshConfig::centered(7.0, 16));
    let hi = mesh(&SphereSdf::new(5.0), &MeshConfig::centered(7.0, 32));
    assert!(
        hi.vertex_count() > lo.vertex_count(),
        "higher resolution should produce more vertices: lo={} hi={}",
        lo.vertex_count(),
        hi.vertex_count()
    );
}

/// All vertices of a sphere mesh should lie near the sphere surface (within one
/// grid-cell half-diagonal of radius 5 at resolution 24 — cell side ≈ 0.58).
#[test]
fn sphere_vertices_near_surface() {
    let r = 5.0_f32;
    let cfg = coarse(7.0);
    let m = mesh(&SphereSdf::new(r), &cfg);
    let cell_diag = (cfg.bounds_max - cfg.bounds_min) / cfg.resolution as f32;
    let tol = cell_diag.length(); // ~1 cell diagonal

    for (idx, v) in m.vertices.iter().enumerate() {
        let dist = (v.length() - r).abs();
        assert!(
            dist < tol,
            "vertex {idx} at {v:?}: distance to surface = {dist:.4}, tol = {tol:.4}"
        );
    }
}

// ── Cube ─────────────────────────────────────────────────────────────────────

#[test]
fn cube_produces_valid_mesh() {
    let m = mesh(&CubeSdf::new(10.0, 10.0, 10.0), &coarse(8.0));
    assert!(m.vertex_count() > 0);
    assert!(m.triangle_count() > 0);
    assert!(m.is_index_valid());
}

/// A box SDF at resolution 24 should produce at least 6 faces × 2 triangles.
#[test]
fn cube_has_enough_triangles() {
    let m = mesh(&CubeSdf::new(10.0, 10.0, 10.0), &coarse(8.0));
    assert!(
        m.triangle_count() >= 12,
        "expected ≥ 12 triangles for a cube, got {}",
        m.triangle_count()
    );
}

// ── Cylinder ──────────────────────────────────────────────────────────────────

#[test]
fn cylinder_produces_valid_mesh() {
    let m = mesh(&CylinderSdf::new(4.0, 8.0), &coarse(7.0));
    assert!(m.vertex_count() > 0);
    assert!(m.triangle_count() > 0);
    assert!(m.is_index_valid());
}

// ── Bounds must contain the surface ──────────────────────────────────────────

/// When the bounding box is too small (surface cut off), the mesher should
/// still return valid indices — just fewer triangles.
#[test]
fn truncated_sphere_still_valid() {
    // Sphere r=5 but bounds only ±3 — caps will be open.
    let cfg = MeshConfig::centered(3.0, 16);
    let m = mesh(&SphereSdf::new(5.0), &cfg);
    assert!(m.is_index_valid());
}

// ── SDF node (trait object) ───────────────────────────────────────────────────

#[test]
fn mesh_from_sdf_node_trait_object() {
    let node: SdfNode = Box::new(SphereSdf::new(5.0));
    let m = mesh(node.as_ref(), &coarse(7.0));
    assert!(m.vertex_count() > 0);
    assert!(m.is_index_valid());
}

// ── CSG union ────────────────────────────────────────────────────────────────

#[test]
fn union_two_spheres_produces_valid_mesh() {
    let a: SdfNode = Box::new(SphereSdf::new(4.0));
    let b: SdfNode = Box::new(SphereSdf::new(4.0));
    // Shift b by translating the SDF query — fake an offset by using a raw
    // closure wrapper isn't possible, so just verify union of two coincident
    // spheres produces a valid mesh.
    let union: SdfNode = Box::new(UnionNode { a, b });
    let m = mesh(union.as_ref(), &coarse(7.0));
    assert!(m.vertex_count() > 0);
    assert!(m.triangle_count() > 0);
    assert!(m.is_index_valid());
}

// ── MeshConfig helpers ────────────────────────────────────────────────────────

#[test]
fn mesh_config_centered_bounds() {
    let cfg = MeshConfig::centered(5.0, 32);
    assert_eq!(cfg.bounds_min, Vec3::splat(-5.0));
    assert_eq!(cfg.bounds_max, Vec3::splat(5.0));
    assert_eq!(cfg.resolution, 32);
}

#[test]
fn mesh_config_default_is_valid() {
    let cfg = MeshConfig::default();
    assert!(cfg.resolution > 0);
    assert!(cfg.bounds_max.x > cfg.bounds_min.x);
}
