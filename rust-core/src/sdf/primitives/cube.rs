//! Axis-aligned box (cube) SDF.
//!
//! Uses the Inigo Quilez exact box SDF formula:
//!
//!   q = |p| - half_extents
//!   d = |max(q, 0)| + min(max(q.x, q.y, q.z), 0)
//!
//! This produces the exact Euclidean distance to the box surface at every
//! point — not just outside (as grid-based formulas do) but also inside,
//! where the value is negative (distance to the nearest face, negated).

use glam::Vec3;

use crate::sdf::Sdf;

/// SDF for an axis-aligned rectangular box centered at the origin.
///
/// Positive outside, negative inside, exactly zero on the surface.
///
/// # Coordinate convention
///
/// The box is centred at the origin.  To place it elsewhere, wrap it in a
/// [`crate::sdf::TransformedSdf`] with a translation transform.
///
/// # Example
/// ```no_run
/// use _core::sdf::primitives::CubeSdf;
/// use _core::sdf::Sdf;
/// use glam::Vec3;
///
/// let cube = CubeSdf::new(10.0, 10.0, 10.0);
/// assert!(cube.distance(Vec3::ZERO) < 0.0);          // inside
/// assert!(cube.distance(Vec3::new(10.0, 0.0, 0.0)) > 0.0); // outside
/// ```
pub struct CubeSdf {
    /// Half the full dimensions along each axis.
    half: Vec3,
}

impl CubeSdf {
    /// Create from full dimensions (width × depth × height).
    ///
    /// The box spans `[-width/2, width/2]` × `[-depth/2, depth/2]`
    ///                                       × `[-height/2, height/2]`.
    #[inline]
    pub fn new(width: f32, depth: f32, height: f32) -> Self {
        Self {
            half: Vec3::new(width * 0.5, depth * 0.5, height * 0.5),
        }
    }

    /// Create directly from half-extents `(hx, hy, hz)`.
    #[inline]
    pub fn from_half_extents(hx: f32, hy: f32, hz: f32) -> Self {
        Self {
            half: Vec3::new(hx, hy, hz),
        }
    }

    /// Return the full dimensions `(width, depth, height)`.
    #[inline]
    pub fn dimensions(&self) -> (f32, f32, f32) {
        (self.half.x * 2.0, self.half.y * 2.0, self.half.z * 2.0)
    }
}

impl Sdf for CubeSdf {
    /// Evaluate the exact signed distance from `p` to the box surface.
    ///
    /// Uses the Quilez formula — the same distance whether inside or outside.
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        // Per-axis: how far outside the slab on this axis (positive = outside).
        let q = p.abs() - self.half;

        // Exterior: length of the clamped-to-positive q vector.
        // Interior: the largest (least negative) per-axis value, which gives
        //           the signed distance to the nearest face (negative).
        q.max(Vec3::ZERO).length() + q.x.max(q.y).max(q.z).min(0.0)
    }

    /// Analytical normal — exact and free of finite-difference error.
    fn normal(&self, p: Vec3) -> Vec3 {
        let q = p.abs() - self.half;
        if q.x > q.y && q.x > q.z {
            Vec3::new(p.x.signum(), 0.0, 0.0)
        } else if q.y > q.z {
            Vec3::new(0.0, p.y.signum(), 0.0)
        } else {
            Vec3::new(0.0, 0.0, p.z.signum())
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    // A 10×10×10 mm cube — half-extents are (5, 5, 5).
    fn unit_cube() -> CubeSdf {
        CubeSdf::new(10.0, 10.0, 10.0)
    }

    #[test]
    fn center_is_inside() {
        let c = unit_cube();
        assert!(c.distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn center_distance_equals_negative_half_extent() {
        // At the centre the nearest face is 5 mm away → distance = -5.
        let c = unit_cube();
        assert_abs_diff_eq!(c.distance(Vec3::ZERO), -5.0, epsilon = 1e-5);
    }

    #[test]
    fn face_center_is_on_surface() {
        let c = unit_cube();
        // Centre of the +X face (5, 0, 0).
        assert_abs_diff_eq!(c.distance(Vec3::new(5.0, 0.0, 0.0)), 0.0, epsilon = 1e-5);
        // Centre of the -Z face (0, 0, -5).
        assert_abs_diff_eq!(c.distance(Vec3::new(0.0, 0.0, -5.0)), 0.0, epsilon = 1e-5);
    }

    #[test]
    fn point_outside_is_positive() {
        let c = unit_cube();
        // 3 mm past the +X face.
        assert_abs_diff_eq!(c.distance(Vec3::new(8.0, 0.0, 0.0)), 3.0, epsilon = 1e-5);
    }

    #[test]
    fn point_outside_corner_is_correct() {
        let c = unit_cube();
        // 2 mm past the corner along the diagonal — distance is sqrt(4+4+4) ≈ 3.464.
        let p = Vec3::new(7.0, 7.0, 7.0);
        let expected = ((7.0_f32 - 5.0).powi(2) * 3.0).sqrt(); // sqrt(12) ≈ 3.464
        assert_abs_diff_eq!(c.distance(p), expected, epsilon = 1e-4);
    }

    #[test]
    fn point_inside_near_face() {
        let c = unit_cube();
        // 1 mm from the +X face, inside → distance = -1.
        assert_abs_diff_eq!(c.distance(Vec3::new(4.0, 0.0, 0.0)), -1.0, epsilon = 1e-5);
    }

    #[test]
    fn non_uniform_box_face() {
        // 20 × 10 × 5 box → half-extents (10, 5, 2.5).
        let b = CubeSdf::new(20.0, 10.0, 5.0);
        assert_abs_diff_eq!(b.distance(Vec3::new(10.0, 0.0, 0.0)), 0.0, epsilon = 1e-5);
        assert_abs_diff_eq!(b.distance(Vec3::new(0.0, 5.0, 0.0)), 0.0, epsilon = 1e-5);
        assert_abs_diff_eq!(b.distance(Vec3::new(0.0, 0.0, 2.5)), 0.0, epsilon = 1e-5);
    }

    #[test]
    fn dimensions_round_trip() {
        let c = CubeSdf::new(20.0, 30.0, 40.0);
        let (w, d, h) = c.dimensions();
        assert_abs_diff_eq!(w, 20.0, epsilon = 1e-5);
        assert_abs_diff_eq!(d, 30.0, epsilon = 1e-5);
        assert_abs_diff_eq!(h, 40.0, epsilon = 1e-5);
    }

    #[test]
    fn normal_points_outward_on_faces() {
        let c = unit_cube();
        // On the +X face, normal should point in +X direction.
        let n = c.normal(Vec3::new(5.0, 0.0, 0.0));
        assert_abs_diff_eq!(n.x, 1.0, epsilon = 1e-5);
        assert_abs_diff_eq!(n.y, 0.0, epsilon = 1e-5);
        assert_abs_diff_eq!(n.z, 0.0, epsilon = 1e-5);

        // On the -Y face, normal should point in -Y direction.
        let n = c.normal(Vec3::new(0.0, -5.0, 0.0));
        assert_abs_diff_eq!(n.x, 0.0, epsilon = 1e-5);
        assert_abs_diff_eq!(n.y, -1.0, epsilon = 1e-5);
        assert_abs_diff_eq!(n.z, 0.0, epsilon = 1e-5);
    }

    #[test]
    fn cube_implements_sdf_trait_object() {
        use crate::sdf::SdfNode;
        let c: SdfNode = Box::new(CubeSdf::new(10.0, 10.0, 10.0));
        assert!(c.distance(Vec3::ZERO) < 0.0);
    }
}
