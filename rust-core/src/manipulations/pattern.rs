//! Spatial pattern manipulation nodes.
//!
//! These nodes replicate an SDF across space by folding the query point into a
//! canonical region before evaluating the inner SDF.  The inner SDF only needs
//! to describe **one** copy; the pattern does the rest.
//!
//! | Node                  | Operation                                  |
//! |-----------------------|--------------------------------------------|
//! | `MirrorNode`          | Mirror (fold) across an axis plane         |
//! | `RepeatLinearNode`    | Infinite or finite 1-D / 3-D tile          |
//! | `RepeatCircularNode`  | N copies arranged around the Z axis        |

use std::f32::consts::{PI, TAU};

use glam::{Vec2, Vec3};

use crate::sdf::{Sdf, SdfNode};

// ── MirrorNode ────────────────────────────────────────────────────────────────

/// Which axis-plane to mirror across.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorAxis {
    /// Mirror across the YZ plane (fold the X coordinate to |x|).
    X,
    /// Mirror across the XZ plane (fold the Y coordinate to |y|).
    Y,
    /// Mirror across the XY plane (fold the Z coordinate to |z|).
    Z,
}

/// Mirror an SDF across a coordinate-axis plane.
///
/// The inner SDF lives on one side (positive half-space); the node
/// symmetrically doubles it across the chosen plane.
///
/// ```text
/// mirror_x(p) = inner(|p.x|, p.y, p.z)
/// ```
pub struct MirrorNode {
    pub inner: SdfNode,
    pub axis: MirrorAxis,
}

impl Sdf for MirrorNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        let q = match self.axis {
            MirrorAxis::X => Vec3::new(p.x.abs(), p.y, p.z),
            MirrorAxis::Y => Vec3::new(p.x, p.y.abs(), p.z),
            MirrorAxis::Z => Vec3::new(p.x, p.y, p.z.abs()),
        };
        self.inner.distance(q)
    }
}

// ── RepeatLinearNode ──────────────────────────────────────────────────────────

/// Tile an SDF in a regular grid pattern.
///
/// `period` sets the repetition spacing per axis.  A zero component means
/// **no** tiling on that axis (the shape is kept as-is along that axis).
///
/// `count` optionally limits the number of copies per axis (half in each
/// direction from the origin).  `0` or `None` per axis means infinite.
///
/// # Example
///
/// Ten spheres of radius 2 spaced 10 mm apart along X:
///
/// ```text
/// RepeatLinearNode {
///     inner:  Box::new(SphereSdf::new(2.0)),
///     period: Vec3::new(10.0, 0.0, 0.0),
///     count:  [10, 0, 0],
/// }
/// ```
pub struct RepeatLinearNode {
    pub inner: SdfNode,
    /// Repetition spacing per axis.  Zero = no repeat on that axis.
    pub period: Vec3,
    /// Max copies in each direction (0 = infinite).
    pub count: [u32; 3],
}

impl RepeatLinearNode {
    /// Build a finite row of N copies along X spaced `spacing` apart.
    pub fn row_x(inner: SdfNode, spacing: f32, n: u32) -> Self {
        Self {
            inner,
            period: Vec3::new(spacing, 0.0, 0.0),
            count: [n, 0, 0],
        }
    }
}

impl Sdf for RepeatLinearNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        let fold = |coord: f32, period: f32, count: u32| -> f32 {
            if period == 0.0 {
                return coord;
            }
            let t = coord / period;
            let rounded = t.round();
            if count == 0 {
                coord - period * rounded
            } else {
                let half = (count / 2) as f32;
                coord - period * rounded.clamp(-half, half)
            }
        };

        let q = Vec3::new(
            fold(p.x, self.period.x, self.count[0]),
            fold(p.y, self.period.y, self.count[1]),
            fold(p.z, self.period.z, self.count[2]),
        );
        self.inner.distance(q)
    }
}

// ── RepeatCircularNode ────────────────────────────────────────────────────────

/// Arrange `count` copies of an SDF uniformly around the **Z axis**.
///
/// The inner SDF describes the **first** copy, centred at angle 0 (along +X).
/// Points are folded into the angular sector `[−π/N, π/N]` before evaluation.
///
/// # Note on orientation
///
/// If the inner shape is itself centred at the origin (e.g. a sphere or box)
/// you should offset it first so that it sits at the desired orbit radius.
/// Use a [`crate::sdf::TransformedSdf`] with a translation along +X.
pub struct RepeatCircularNode {
    pub inner: SdfNode,
    /// Number of copies (minimum 1).
    pub count: u32,
}

impl Sdf for RepeatCircularNode {
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        let n = self.count.max(1) as f32;
        let an = TAU / n; // full sector angle
        let han = PI / n; // half sector angle

        let r_xy = Vec2::new(p.x, p.y).length();
        let angle = p.y.atan2(p.x);

        // Fold into [−π/N, π/N] — same logic as the n-gon prism SDF.
        let folded = (angle + han).rem_euclid(an) - han;

        let q = Vec3::new(r_xy * folded.cos(), r_xy * folded.sin(), p.z);
        self.inner.distance(q)
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::primitives::SphereSdf;
    use approx::assert_abs_diff_eq;

    fn sphere2() -> SdfNode {
        Box::new(SphereSdf::new(2.0))
    }

    // ── MirrorNode ─────────────────────────────────────────────────────────

    #[test]
    fn mirror_x_reflects_positive_and_negative() {
        // A sphere at (0,0,0) mirrored across X has the same distance from +X and -X.
        let sdf = MirrorNode {
            inner: sphere2(),
            axis: MirrorAxis::X,
        };
        let d_pos = sdf.distance(Vec3::new(3.0, 0.0, 0.0));
        let d_neg = sdf.distance(Vec3::new(-3.0, 0.0, 0.0));
        assert_abs_diff_eq!(d_pos, d_neg, epsilon = 1e-5);
    }

    #[test]
    fn mirror_z_makes_plane_symmetric() {
        let sdf = MirrorNode {
            inner: sphere2(),
            axis: MirrorAxis::Z,
        };
        let d_pos = sdf.distance(Vec3::new(0.0, 0.0, 1.0));
        let d_neg = sdf.distance(Vec3::new(0.0, 0.0, -1.0));
        assert_abs_diff_eq!(d_pos, d_neg, epsilon = 1e-5);
    }

    #[test]
    fn mirror_y_inside_is_inside() {
        let sdf = MirrorNode {
            inner: sphere2(),
            axis: MirrorAxis::Y,
        };
        assert!(sdf.distance(Vec3::ZERO) < 0.0);
    }

    // ── RepeatLinearNode ───────────────────────────────────────────────────

    #[test]
    fn repeat_infinite_x_copies_on_axis() {
        let sdf = RepeatLinearNode {
            inner: sphere2(),
            period: Vec3::new(10.0, 0.0, 0.0),
            count: [0, 0, 0],
        };
        // Sphere centred at x=0 and x=±10, etc.  Point at x=10 is at a copy centre.
        assert!(
            sdf.distance(Vec3::new(10.0, 0.0, 0.0)) < 0.0,
            "copy at x=10 should be inside"
        );
        assert!(
            sdf.distance(Vec3::new(20.0, 0.0, 0.0)) < 0.0,
            "copy at x=20 should be inside"
        );
        assert!(
            sdf.distance(Vec3::new(0.0, 0.0, 0.0)) < 0.0,
            "original should be inside"
        );
    }

    #[test]
    fn repeat_between_copies_is_outside() {
        let sdf = RepeatLinearNode {
            inner: sphere2(),
            period: Vec3::new(10.0, 0.0, 0.0),
            count: [0, 0, 0],
        };
        // Midpoint between copies at x=5 should be outside (sphere r=2, nearest copy 5 away).
        assert!(sdf.distance(Vec3::new(5.0, 0.0, 0.0)) > 0.0);
    }

    #[test]
    fn repeat_finite_does_not_tile_beyond_count() {
        // 3 copies (count[0]=3 → -1..=1 in units of period=10 → at x=-10,0,+10).
        let sdf = RepeatLinearNode {
            inner: sphere2(),
            period: Vec3::new(10.0, 0.0, 0.0),
            count: [3, 0, 0],
        };
        // x=30 should be clamped to the last copy at x=10 → distance = 30-10-2 = 18.
        let d = sdf.distance(Vec3::new(30.0, 0.0, 0.0));
        assert!(d > 0.0, "beyond finite count should be outside, got {d}");
        // x=10 should be inside (last copy).
        assert!(sdf.distance(Vec3::new(10.0, 0.0, 0.0)) < 0.0);
    }

    // ── RepeatCircularNode ─────────────────────────────────────────────────

    #[test]
    fn repeat_circular_all_copies_equidistant_from_center() {
        // One sphere of r=1 at (5,0,0) repeated 6 times around Z.
        // The SDF folds each point into the first sector, so all 6 positions
        // at distance 5 from Z axis should read the same SDF value.
        use crate::math::Transform;
        use crate::sdf::TransformedSdf;

        let sdf = RepeatCircularNode {
            inner: Box::new(TransformedSdf {
                inner: Box::new(SphereSdf::new(1.0)),
                transform: Transform::from_translation(Vec3::new(5.0, 0.0, 0.0)),
            }),
            count: 6,
        };

        let base_d = sdf.distance(Vec3::new(5.0, 0.0, 0.0));
        for k in 1..6u32 {
            let angle = k as f32 * TAU / 6.0;
            let p = Vec3::new(5.0 * angle.cos(), 5.0 * angle.sin(), 0.0);
            let d = sdf.distance(p);
            assert_abs_diff_eq!(d, base_d, epsilon = 1e-4);
        }
    }

    #[test]
    fn repeat_circular_count_1_is_identity() {
        // Count=1 → full TAU sector, fold is identity.
        let sdf = RepeatCircularNode {
            inner: sphere2(),
            count: 1,
        };
        assert_abs_diff_eq!(
            sdf.distance(Vec3::ZERO),
            SphereSdf::new(2.0).distance(Vec3::ZERO),
            epsilon = 1e-4,
        );
    }
}
