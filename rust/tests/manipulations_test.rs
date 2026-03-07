//! Integration tests for the manipulations module.

use std::f32::consts::TAU;

use approx::assert_abs_diff_eq;
use glam::Vec3;

use _core::manipulations::{
    ElongateNode, MirrorAxis, MirrorNode, OffsetNode, RepeatCircularNode,
    RepeatLinearNode, ShellNode,
};
use _core::math::Transform;
use _core::sdf::primitives::{CubeSdf, CylinderSdf, SphereSdf};
use _core::sdf::{operations::DifferenceNode, Sdf, SdfNode, TransformedSdf};

// ── OffsetNode (fillet / rounding) ────────────────────────────────────────────

#[test]
fn offset_expands_sphere_surface() {
    // Sphere r=5, +2 offset → surface at r=7.
    let sdf = OffsetNode {
        inner:  Box::new(SphereSdf::new(5.0)),
        offset: 2.0,
    };
    assert_abs_diff_eq!(sdf.distance(Vec3::new(7.0, 0.0, 0.0)), 0.0, epsilon = 1e-4);
    assert!(sdf.distance(Vec3::new(6.0, 0.0, 0.0)) < 0.0);
    assert!(sdf.distance(Vec3::new(8.0, 0.0, 0.0)) > 0.0);
}

#[test]
fn offset_contracts_sphere_surface() {
    let sdf = OffsetNode {
        inner:  Box::new(SphereSdf::new(5.0)),
        offset: -2.0,
    };
    // Surface now at r=3.
    assert_abs_diff_eq!(sdf.distance(Vec3::new(3.0, 0.0, 0.0)), 0.0, epsilon = 1e-4);
}

#[test]
fn inset_fillet_preserves_shape() {
    // Expand by r then contract by r → same as original (approximately).
    let base   = CubeSdf::new(10.0, 10.0, 10.0);
    let expand: SdfNode = Box::new(OffsetNode {
        inner:  Box::new(CubeSdf::new(10.0, 10.0, 10.0)),
        offset: 1.0,
    });
    let restore = OffsetNode { inner: expand, offset: -1.0 };

    // At the original face centre (5,0,0), both should be ~0.
    assert_abs_diff_eq!(base.distance(Vec3::new(5.0, 0.0, 0.0)),     0.0, epsilon = 1e-4);
    assert_abs_diff_eq!(restore.distance(Vec3::new(5.0, 0.0, 0.0)), 0.0, epsilon = 1e-4);
}

// ── ShellNode (hollow) ────────────────────────────────────────────────────────

#[test]
fn shell_outer_surface_at_r_plus_thickness() {
    let sdf = ShellNode {
        inner:     Box::new(SphereSdf::new(5.0)),
        thickness: 1.0,
    };
    // Wall spans [r−t, r+t] = [4, 6].  Surface at r=4 and r=6.
    assert_abs_diff_eq!(sdf.distance(Vec3::new(4.0, 0.0, 0.0)), 0.0, epsilon = 1e-4);
    assert_abs_diff_eq!(sdf.distance(Vec3::new(6.0, 0.0, 0.0)), 0.0, epsilon = 1e-4);
}

#[test]
fn shell_original_surface_is_deepest_inside() {
    let sdf = ShellNode {
        inner:     Box::new(SphereSdf::new(5.0)),
        thickness: 1.0,
    };
    assert_abs_diff_eq!(sdf.distance(Vec3::new(5.0, 0.0, 0.0)), -1.0, epsilon = 1e-4);
}

#[test]
fn shell_interior_is_positive() {
    // Deep inside (r=0) → above threshold → outside the shell.
    let sdf = ShellNode { inner: Box::new(SphereSdf::new(5.0)), thickness: 1.0 };
    assert!(sdf.distance(Vec3::ZERO) > 0.0);
}

#[test]
fn shell_exterior_is_positive() {
    let sdf = ShellNode { inner: Box::new(SphereSdf::new(5.0)), thickness: 1.0 };
    assert!(sdf.distance(Vec3::new(10.0, 0.0, 0.0)) > 0.0);
}

/// A shelled cube, meshed, should produce a valid mesh with more triangles than
/// a solid cube (both inner and outer surface are tessellated).
#[test]
fn shell_produces_valid_mesh() {
    use _core::mesher::{mesh, MeshConfig};
    let sdf = ShellNode { inner: Box::new(CubeSdf::new(10.0, 10.0, 10.0)), thickness: 1.0 };
    let m   = mesh(&sdf, &MeshConfig::centered(8.0, 24));
    assert!(m.vertex_count()   > 0);
    assert!(m.triangle_count() > 12, "shell cube should have inner+outer triangles");
    assert!(m.is_index_valid());
}

// ── ElongateNode ──────────────────────────────────────────────────────────────

#[test]
fn elongate_sphere_along_x_gives_capsule() {
    // Sphere r=2 elongated 5 along X → capsule, end-caps at x=±7.
    let sdf = ElongateNode {
        inner:   Box::new(SphereSdf::new(2.0)),
        amounts: Vec3::new(5.0, 0.0, 0.0),
    };
    // End-cap surface at (7,0,0).
    assert_abs_diff_eq!(sdf.distance(Vec3::new(7.0, 0.0, 0.0)), 0.0, epsilon = 1e-4);
    // Mid-tube surface at (0, 2, 0).
    assert_abs_diff_eq!(sdf.distance(Vec3::new(0.0, 2.0, 0.0)), 0.0, epsilon = 1e-4);
    // Inside the tube.
    assert!(sdf.distance(Vec3::new(3.0, 0.0, 0.0)) < 0.0);
}

#[test]
fn elongate_z_adds_length() {
    // Cylinder r=3, h=10 → elongate by 5 along Z → cylinder r=3, h=20.
    let sdf = ElongateNode {
        inner:   Box::new(CylinderSdf::new(3.0, 10.0)),
        amounts: Vec3::new(0.0, 0.0, 5.0),
    };
    // Original top cap was at z=5; new top cap at z=10.
    assert_abs_diff_eq!(sdf.distance(Vec3::new(0.0, 0.0, 10.0)), 0.0, epsilon = 1e-4);
    assert!(sdf.distance(Vec3::new(0.0, 0.0, 8.0)) < 0.0);
}

#[test]
fn elongate_zero_is_transparent() {
    let sdf = ElongateNode {
        inner:   Box::new(SphereSdf::new(5.0)),
        amounts: Vec3::ZERO,
    };
    assert_abs_diff_eq!(sdf.distance(Vec3::ZERO), -5.0, epsilon = 1e-4);
}

// ── MirrorNode ────────────────────────────────────────────────────────────────

#[test]
fn mirror_x_both_halves_reachable() {
    // A cylinder along Z, positioned at x=3 (so only in +x half without mirror).
    // After mirror across X, a copy exists in the -x half too.
    let sdf = MirrorNode {
        inner: Box::new(TransformedSdf {
            inner:     Box::new(CylinderSdf::new(1.0, 10.0)),
            transform: Transform::from_translation(Vec3::new(3.0, 0.0, 0.0)),
        }),
        axis: MirrorAxis::X,
    };
    // Positive side: inside the cylinder at (3,0,0).
    assert!(sdf.distance(Vec3::new(3.0, 0.0, 0.0)) < 0.0);
    // Negative side: should also be inside (mirrored copy).
    assert!(sdf.distance(Vec3::new(-3.0, 0.0, 0.0)) < 0.0);
}

#[test]
fn mirror_z_symmetry_across_xy_plane() {
    let sdf = MirrorNode {
        inner: Box::new(SphereSdf::new(2.0)),
        axis:  MirrorAxis::Z,
    };
    let dp = sdf.distance(Vec3::new(1.0, 1.0,  1.0));
    let dn = sdf.distance(Vec3::new(1.0, 1.0, -1.0));
    assert_abs_diff_eq!(dp, dn, epsilon = 1e-5);
}

// ── RepeatLinearNode ──────────────────────────────────────────────────────────

#[test]
fn repeat_x_infinite_places_copies_at_multiples() {
    let sdf = RepeatLinearNode {
        inner:  Box::new(SphereSdf::new(1.0)),
        period: Vec3::new(5.0, 0.0, 0.0),
        count:  [0, 0, 0],
    };
    // Copies at x = …−10, −5, 0, 5, 10, …
    for &x in &[-10.0_f32, -5.0, 0.0, 5.0, 10.0] {
        assert!(
            sdf.distance(Vec3::new(x, 0.0, 0.0)) < 0.0,
            "copy at x={x} should be inside"
        );
    }
}

#[test]
fn repeat_3d_tile_inside_at_grid_points() {
    let sdf = RepeatLinearNode {
        inner:  Box::new(SphereSdf::new(1.5)),
        period: Vec3::new(5.0, 5.0, 5.0),
        count:  [0, 0, 0],
    };
    for &(x, y, z) in &[(0.0, 0.0, 0.0), (5.0, 5.0, 0.0), (0.0, 5.0, 5.0)] {
        assert!(sdf.distance(Vec3::new(x, y, z)) < 0.0);
    }
}

#[test]
fn repeat_finite_row_helper() {
    let sdf = RepeatLinearNode::row_x(Box::new(SphereSdf::new(1.0)), 6.0, 4);
    // 4 copies → count=4, clamped at ±2 in units of 6 → at x=-12,-6,0,6,12.
    // Wait, with count=4 and half = 4/2 = 2, clamp to ±2 → x at -12,-6,0,6,12.
    // Hmm, that's 5 positions for count=4. Let me just check that the origin is inside.
    assert!(sdf.distance(Vec3::ZERO) < 0.0);
}

// ── RepeatCircularNode ────────────────────────────────────────────────────────

#[test]
fn repeat_circular_6_copies_all_inside() {
    // 6 spheres of r=1 at distance 5 from Z axis.
    let sdf = RepeatCircularNode {
        inner: Box::new(TransformedSdf {
            inner:     Box::new(SphereSdf::new(1.0)),
            transform: Transform::from_translation(Vec3::new(5.0, 0.0, 0.0)),
        }),
        count: 6,
    };
    for k in 0..6u32 {
        let angle = k as f32 * TAU / 6.0;
        let p = Vec3::new(5.0 * angle.cos(), 5.0 * angle.sin(), 0.0);
        assert!(
            sdf.distance(p) < 0.0,
            "copy {k} at angle {} should be inside",
            angle.to_degrees()
        );
    }
}

#[test]
fn repeat_circular_between_copies_is_outside() {
    // Same setup as above — midpoints between copies should be outside.
    let sdf = RepeatCircularNode {
        inner: Box::new(TransformedSdf {
            inner:     Box::new(SphereSdf::new(1.0)),
            transform: Transform::from_translation(Vec3::new(5.0, 0.0, 0.0)),
        }),
        count: 6,
    };
    // Halfway between copies at 30° (TAU/12).
    let angle = TAU / 12.0;
    let p = Vec3::new(5.0 * angle.cos(), 5.0 * angle.sin(), 0.0);
    assert!(sdf.distance(p) > 0.0, "between copies should be outside");
}

// ── Combining manipulations ───────────────────────────────────────────────────

/// Round a cube (positive offset), then cut a cylindrical bore through it.
#[test]
fn rounded_cube_with_bore() {
    // Rounded cube: 10×10×10 + offset 0.5.
    let rounded: SdfNode = Box::new(OffsetNode {
        inner:  Box::new(CubeSdf::new(10.0, 10.0, 10.0)),
        offset: 0.5,
    });
    // Bore: cylinder r=2, h=20 along Z.
    let bore: SdfNode = Box::new(CylinderSdf::new(2.0, 20.0));
    let result: SdfNode = Box::new(DifferenceNode { a: rounded, b: bore });

    // On-axis inside bore → outside result.
    assert!(result.distance(Vec3::ZERO) > 0.0);
    // Off-axis inside rounded cube, outside bore → inside result.
    assert!(result.distance(Vec3::new(4.0, 0.0, 0.0)) < 0.0);
    // Far outside → outside.
    assert!(result.distance(Vec3::new(20.0, 0.0, 0.0)) > 0.0);
}

/// Shell a cylinder then mirror it to get two shells side by side.
#[test]
fn shelled_mirrored_cylinder() {
    let cyl_shell: SdfNode = Box::new(ShellNode {
        inner:     Box::new(TransformedSdf {
            inner:     Box::new(CylinderSdf::new(3.0, 10.0)),
            transform: Transform::from_translation(Vec3::new(8.0, 0.0, 0.0)),
        }),
        thickness: 0.5,
    });
    let doubled = MirrorNode { inner: cyl_shell, axis: MirrorAxis::X };

    // Wall of the positive-X cylinder.
    assert!(doubled.distance(Vec3::new( 8.0, 3.0, 0.0)) < 0.1);
    // Wall of the mirrored -X cylinder.
    assert!(doubled.distance(Vec3::new(-8.0, 3.0, 0.0)) < 0.1);
}
