//! Torus SDF.
//!
//! The torus is centred at the origin and lies in the **XY plane** (the tube
//! circles around the Z axis).
//!
//! Exact SDF formula (one of the simplest in the SDF catalogue):
//!
//!   q = (length(p.xy) - R,  p.z)
//!   d = length(q) - r
//!
//! Where:
//!   R = major radius — distance from origin to the centre of the tube.
//!   r = minor radius — radius of the tube cross-section.
//!
//! The formula works by "unrolling" the torus into a 2-D circle problem:
//! first find the nearest point on the ring (the circle of radius R in XY),
//! then measure the distance from that ring to p, then subtract r.

use glam::Vec2;
use glam::Vec3;

use crate::sdf::Sdf;

/// SDF for a torus centred at the origin, ring in the XY plane.
///
/// `ph.Object(ph.TORUS, major_radius, minor_radius)` maps to
/// `TorusSdf::new(major_radius, minor_radius)`.
///
/// # Parameters
/// - `major_radius` (R): distance from the origin to the centre of the tube.
/// - `minor_radius` (r): radius of the circular tube cross-section.
///
/// The torus has an inner radius of `R - r` and an outer radius of `R + r`.
/// It requires `R > r > 0` for a non-degenerate shape.
pub struct TorusSdf {
    major_radius: f32, // R
    minor_radius: f32, // r
}

impl TorusSdf {
    /// Create from major radius `R` and minor radius `r`.
    #[inline]
    pub fn new(major_radius: f32, minor_radius: f32) -> Self {
        Self {
            major_radius,
            minor_radius,
        }
    }

    /// Return `(major_radius, minor_radius)`.
    #[inline]
    pub fn radii(&self) -> (f32, f32) {
        (self.major_radius, self.minor_radius)
    }

    /// Outer radius of the torus bounding circle: `R + r`.
    #[inline]
    pub fn outer_radius(&self) -> f32 {
        self.major_radius + self.minor_radius
    }

    /// Inner radius (the hole): `R - r`. May be zero or negative for fat tori.
    #[inline]
    pub fn inner_radius(&self) -> f32 {
        self.major_radius - self.minor_radius
    }
}

impl Sdf for TorusSdf {
    /// Exact signed distance from `p` to the torus surface.
    ///
    /// Negative inside the tube, zero on the surface, positive outside.
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        // Radial distance in XY, offset by major radius — gives the nearest
        // point on the ring circle projected into a 1-D value.
        let q = Vec2::new(Vec2::new(p.x, p.y).length() - self.major_radius, p.z);
        q.length() - self.minor_radius
    }

    /// Exact analytical normal.
    ///
    /// The nearest point on the ring from `p` is `(p.x, p.y)` normalised then
    /// scaled to major_radius. The normal is the unit vector from that point to `p`.
    fn normal(&self, p: Vec3) -> Vec3 {
        let r_xy = Vec2::new(p.x, p.y).length().max(f32::EPSILON);
        // Closest point on the central ring.
        let ring = Vec3::new(
            p.x / r_xy * self.major_radius,
            p.y / r_xy * self.major_radius,
            0.0,
        );
        (p - ring).normalize_or_zero()
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    /// R=5, r=2.  Inner radius=3, outer=7, tube centre at (5,0,0).
    fn torus() -> TorusSdf {
        TorusSdf::new(5.0, 2.0)
    }

    // ── Accessors ─────────────────────────────────────────────────────────

    #[test]
    fn radii_round_trip() {
        let (r, m) = torus().radii();
        assert_abs_diff_eq!(r, 5.0, epsilon = 1e-5);
        assert_abs_diff_eq!(m, 2.0, epsilon = 1e-5);
    }

    #[test]
    fn outer_radius_is_sum() {
        assert_abs_diff_eq!(torus().outer_radius(), 7.0, epsilon = 1e-5);
    }

    #[test]
    fn inner_radius_is_difference() {
        assert_abs_diff_eq!(torus().inner_radius(), 3.0, epsilon = 1e-5);
    }

    // ── Origin (inside the hole) ───────────────────────────────────────────

    #[test]
    fn origin_is_outside_in_the_hole() {
        // d(origin) = length((0 - 5, 0)) - 2 = 5 - 2 = 3 > 0.
        assert_abs_diff_eq!(torus().distance(Vec3::ZERO), 3.0, epsilon = 1e-5);
    }

    // ── Tube centre ────────────────────────────────────────────────────────

    #[test]
    fn tube_centre_is_inside() {
        // Tube centre is at (R, 0, 0) = (5, 0, 0).
        // d = length((5-5, 0)) - 2 = 0 - 2 = -2.
        assert_abs_diff_eq!(
            torus().distance(Vec3::new(5.0, 0.0, 0.0)),
            -2.0,
            epsilon = 1e-5
        );
    }

    // ── Surface points ─────────────────────────────────────────────────────

    #[test]
    fn outer_surface_is_zero() {
        // Outermost point: (R+r, 0, 0) = (7, 0, 0).
        assert_abs_diff_eq!(
            torus().distance(Vec3::new(7.0, 0.0, 0.0)),
            0.0,
            epsilon = 1e-5
        );
    }

    #[test]
    fn inner_surface_is_zero() {
        // Innermost point: (R-r, 0, 0) = (3, 0, 0).
        assert_abs_diff_eq!(
            torus().distance(Vec3::new(3.0, 0.0, 0.0)),
            0.0,
            epsilon = 1e-5
        );
    }

    #[test]
    fn top_of_tube_is_zero() {
        // Top of tube (along +Z from tube centre): (R, 0, r) = (5, 0, 2).
        assert_abs_diff_eq!(
            torus().distance(Vec3::new(5.0, 0.0, 2.0)),
            0.0,
            epsilon = 1e-5
        );
    }

    #[test]
    fn bottom_of_tube_is_zero() {
        assert_abs_diff_eq!(
            torus().distance(Vec3::new(5.0, 0.0, -2.0)),
            0.0,
            epsilon = 1e-5
        );
    }

    #[test]
    fn surface_is_rotationally_symmetric() {
        // Surface should be zero at (R+r) rotated to any XY angle.
        let t = torus();
        for angle_deg in [0.0_f32, 45.0, 90.0, 135.0, 180.0, 270.0] {
            let angle = angle_deg.to_radians();
            let p = Vec3::new((5.0 + 2.0) * angle.cos(), (5.0 + 2.0) * angle.sin(), 0.0);
            let d = t.distance(p);
            assert!(d.abs() < 1e-5, "angle {angle_deg}°: expected ~0, got {d}");
        }
    }

    // ── Exterior ──────────────────────────────────────────────────────────

    #[test]
    fn outside_above_tube_is_positive() {
        // 3 mm above the top of the tube: (5, 0, 5) → d = length((0,5)) - 2 = 3.
        assert_abs_diff_eq!(
            torus().distance(Vec3::new(5.0, 0.0, 5.0)),
            3.0,
            epsilon = 1e-5
        );
    }

    #[test]
    fn outside_past_outer_rim_is_positive() {
        // 2 mm past the outer surface: (9, 0, 0) → d = length((4,0)) - 2 = 2.
        assert_abs_diff_eq!(
            torus().distance(Vec3::new(9.0, 0.0, 0.0)),
            2.0,
            epsilon = 1e-5
        );
    }

    // ── Normal ────────────────────────────────────────────────────────────

    #[test]
    fn normal_on_outer_surface_points_outward_radially() {
        let n = torus().normal(Vec3::new(7.0, 0.0, 0.0));
        assert_abs_diff_eq!(n.x, 1.0, epsilon = 1e-4);
        assert_abs_diff_eq!(n.y, 0.0, epsilon = 1e-4);
        assert_abs_diff_eq!(n.z, 0.0, epsilon = 1e-4);
    }

    #[test]
    fn normal_on_top_of_tube_points_up() {
        let n = torus().normal(Vec3::new(5.0, 0.0, 2.0));
        assert_abs_diff_eq!(n.x, 0.0, epsilon = 1e-4);
        assert_abs_diff_eq!(n.y, 0.0, epsilon = 1e-4);
        assert_abs_diff_eq!(n.z, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn normal_on_inner_surface_points_inward() {
        // Inner surface at (3, 0, 0): normal should point in -X direction.
        let n = torus().normal(Vec3::new(3.0, 0.0, 0.0));
        assert_abs_diff_eq!(n.x, -1.0, epsilon = 1e-4);
        assert_abs_diff_eq!(n.y, 0.0, epsilon = 1e-4);
        assert_abs_diff_eq!(n.z, 0.0, epsilon = 1e-4);
    }

    // ── Trait object ──────────────────────────────────────────────────────

    #[test]
    fn torus_as_sdf_node() {
        use crate::sdf::SdfNode;
        let node: SdfNode = Box::new(TorusSdf::new(5.0, 2.0));
        // Tube centre is inside.
        assert!(node.distance(Vec3::new(5.0, 0.0, 0.0)) < 0.0);
        // Origin (in hole) is outside.
        assert!(node.distance(Vec3::ZERO) > 0.0);
    }
}
