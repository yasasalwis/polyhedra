//! Shape-morphing manipulation nodes.
//!
//! These nodes modify the metric of an existing SDF without changing its
//! topological type.  All operations are exact or Lipschitz-continuous and
//! are safe to use inside the Dual Contouring mesher.
//!
//! | Node          | Operation                          | Formula                     |
//! |---------------|------------------------------------|-----------------------------|
//! | `OffsetNode`  | Uniform expansion / contraction    | `sdf(p) − δ`                |
//! | `ShellNode`   | Hollow shell of given thickness    | `|sdf(p)| − t`              |
//! | `ElongateNode`| Stretch shape along one or more    | `sdf(p − clamp(p, −h, h))`  |
//! |               | axes by pushing empty space        |                             |
//!
//! # Fillet / rounding
//! `OffsetNode` with a **positive** offset implements edge rounding (the
//! Minkowski sum with a ball of radius `δ`): every sharp edge becomes a
//! circular arc of radius `δ`.  The shape grows by `δ` in all directions.
//! Shrinking back with a second `OffsetNode { offset: -δ }` gives an
//! *inset fillet* that preserves the original footprint.
//!
//! # Shell
//! `ShellNode` hollows out any closed solid, leaving a thin wall of the
//! requested thickness.  The resulting SDF is negative only within the wall
//! itself.

use glam::Vec3;

use crate::sdf::{Sdf, SdfNode};

// ── OffsetNode ────────────────────────────────────────────────────────────────

/// Uniformly expand (`offset > 0`) or contract (`offset < 0`) a shape.
///
/// Equivalent to the Minkowski sum with a ball of radius `offset`.
/// A positive offset rounds all sharp edges and corners.
///
/// ```text
/// offset_sdf(p) = inner(p) − offset
/// ```
pub struct OffsetNode {
    pub inner: SdfNode,
    /// Signed offset in model units (mm).  Positive = grow, negative = shrink.
    pub offset: f32,
}

impl Sdf for OffsetNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        self.inner.distance(p) - self.offset
    }
}

// ── ShellNode ─────────────────────────────────────────────────────────────────

/// Hollow out a solid, leaving a shell of the given thickness.
///
/// The result is negative only within `thickness / 2` of the original surface
/// (both inside and outside).
///
/// ```text
/// shell_sdf(p) = |inner(p)| − thickness
/// ```
pub struct ShellNode {
    pub inner: SdfNode,
    /// Wall thickness in model units.
    pub thickness: f32,
}

impl Sdf for ShellNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        self.inner.distance(p).abs() - self.thickness
    }
}

// ── ElongateNode ──────────────────────────────────────────────────────────────

/// Stretch a shape along one or more axes without deforming its cross-section.
///
/// Each non-zero component of `amounts` adds that much extra length in both
/// the positive and negative direction along that axis.  Setting `amounts.y`
/// to `3.0` turns a sphere of radius `r` into a capsule of length `6 + 2r`.
///
/// ```text
/// q = p − clamp(p, −amounts, amounts)
/// elongate_sdf(p) = inner(q)
/// ```
///
/// The clamping pushes the query point into the nearest end-cap region,
/// effectively inserting a straight "tube" of the requested extra length.
pub struct ElongateNode {
    pub inner: SdfNode,
    /// Per-axis elongation in model units.  Zero on an axis = no elongation.
    pub amounts: Vec3,
}

impl Sdf for ElongateNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        let q = p - p.clamp(-self.amounts, self.amounts);
        self.inner.distance(q)
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::primitives::{CubeSdf, SphereSdf};
    use approx::assert_abs_diff_eq;

    fn sphere5() -> SdfNode {
        Box::new(SphereSdf::new(5.0))
    }
    fn cube10() -> SdfNode {
        Box::new(CubeSdf::new(10.0, 10.0, 10.0))
    }

    // ── OffsetNode ─────────────────────────────────────────────────────────

    #[test]
    fn offset_positive_expands_sphere() {
        // Sphere r=5, offset +2 → effective r=7.
        let sdf = OffsetNode {
            inner: sphere5(),
            offset: 2.0,
        };
        // Point at distance 6 from origin: inside original sphere boundary
        // but still inside expanded one? No, r=7, p=6 → inside. d = 6 - 7 = -1.
        assert_abs_diff_eq!(sdf.distance(Vec3::new(6.0, 0.0, 0.0)), -1.0, epsilon = 1e-4);
    }

    #[test]
    fn offset_negative_shrinks_sphere() {
        // Sphere r=5, offset -2 → effective r=3.
        let sdf = OffsetNode {
            inner: sphere5(),
            offset: -2.0,
        };
        // Point at r=4: outside shrunk sphere (r=3). d = 4 - 3 = 1.
        assert_abs_diff_eq!(sdf.distance(Vec3::new(4.0, 0.0, 0.0)), 1.0, epsilon = 1e-4);
    }

    #[test]
    fn offset_surface_shifted_outward() {
        // After +2 offset, surface of sphere is at r=7.
        let sdf = OffsetNode {
            inner: sphere5(),
            offset: 2.0,
        };
        assert_abs_diff_eq!(sdf.distance(Vec3::new(7.0, 0.0, 0.0)), 0.0, epsilon = 1e-4);
    }

    #[test]
    fn offset_rounds_cube_corners() {
        // A cube with positive offset should have corners further from the
        // un-offset surface (the offset "inflates" the shape).
        let raw = CubeSdf::new(10.0, 10.0, 10.0);
        let rounded = OffsetNode {
            inner: cube10(),
            offset: 1.0,
        };

        // Corner of original cube at (5,5,5): raw SDF = 0 (on surface).
        // Rounded SDF at same point = 0 - 1.0 = -1.0 (inside rounded shape).
        let raw_d = raw.distance(Vec3::new(5.0, 5.0, 5.0));
        let rounded_d = rounded.distance(Vec3::new(5.0, 5.0, 5.0));
        assert_abs_diff_eq!(raw_d, 0.0, epsilon = 1e-4);
        assert_abs_diff_eq!(rounded_d, -1.0, epsilon = 1e-4);
    }

    // ── ShellNode ──────────────────────────────────────────────────────────

    #[test]
    fn shell_surface_is_near_zero() {
        // Shell of sphere r=5, thickness 1.  Surface of shell is the original
        // sphere surface (d=0 → |0| - 1 = -1 ... wait, the midpoint of the
        // wall is where |sdf| = thickness, i.e. sdf = ±thickness.
        // Actually surface of shell sdf = 0 where |inner_sdf| = thickness.
        // inner_sdf at r=4: d = 4-5 = -1.  |d|=1, shell_d = 1-1 = 0. ✓
        let sdf = ShellNode {
            inner: sphere5(),
            thickness: 1.0,
        };
        assert_abs_diff_eq!(sdf.distance(Vec3::new(4.0, 0.0, 0.0)), 0.0, epsilon = 1e-4);
        // Also at r=6: d = 6-5 = 1. |d|=1, shell_d = 0. ✓
        assert_abs_diff_eq!(sdf.distance(Vec3::new(6.0, 0.0, 0.0)), 0.0, epsilon = 1e-4);
    }

    #[test]
    fn shell_midpoint_is_most_negative() {
        // At r=5 (original surface), |sdf| = 0, shell_d = -thickness = -1.
        let sdf = ShellNode {
            inner: sphere5(),
            thickness: 1.0,
        };
        assert_abs_diff_eq!(sdf.distance(Vec3::new(5.0, 0.0, 0.0)), -1.0, epsilon = 1e-4);
    }

    #[test]
    fn shell_interior_is_positive() {
        // Deep inside the sphere (r=0), sdf=-5, |sdf|=5, shell_d = 5-1 = 4 > 0.
        let sdf = ShellNode {
            inner: sphere5(),
            thickness: 1.0,
        };
        assert!(sdf.distance(Vec3::ZERO) > 0.0);
    }

    #[test]
    fn shell_exterior_is_positive_beyond_thickness() {
        // At r=7: sdf=2, |sdf|=2, shell_d = 2-1 = 1 > 0.
        let sdf = ShellNode {
            inner: sphere5(),
            thickness: 1.0,
        };
        assert!(sdf.distance(Vec3::new(7.0, 0.0, 0.0)) > 0.0);
    }

    // ── ElongateNode ───────────────────────────────────────────────────────

    #[test]
    fn elongate_sphere_along_x_makes_capsule() {
        // Elongate sphere r=2 by 3 units along X → capsule.
        // Point at (5,0,0): q = (5 - clamp(5,-3,3), 0, 0) = (5-3, 0, 0) = (2,0,0).
        // inner.distance((2,0,0)) = 2-2 = 0. On surface ✓
        let sdf = ElongateNode {
            inner: Box::new(SphereSdf::new(2.0)),
            amounts: Vec3::new(3.0, 0.0, 0.0),
        };
        assert_abs_diff_eq!(sdf.distance(Vec3::new(5.0, 0.0, 0.0)), 0.0, epsilon = 1e-4);
    }

    #[test]
    fn elongate_zero_is_identity() {
        // No elongation → same as original sphere.
        let sdf = ElongateNode {
            inner: sphere5(),
            amounts: Vec3::ZERO,
        };
        assert_abs_diff_eq!(sdf.distance(Vec3::ZERO), -5.0, epsilon = 1e-4);
        assert_abs_diff_eq!(sdf.distance(Vec3::new(5.0, 0.0, 0.0)), 0.0, epsilon = 1e-4);
    }

    #[test]
    fn elongate_inside_the_tube_is_negative() {
        // Elongate sphere r=2 by 10 along X. Origin should be inside.
        let sdf = ElongateNode {
            inner: Box::new(SphereSdf::new(2.0)),
            amounts: Vec3::new(10.0, 0.0, 0.0),
        };
        assert!(sdf.distance(Vec3::ZERO) < 0.0);
        assert!(sdf.distance(Vec3::new(8.0, 0.0, 0.0)) < 0.0);
    }
}
