//! Sphere SDF.
//!
//! The sphere is the simplest possible exact SDF:
//!
//!   d(p) = |p| - radius
//!
//! Positive outside, negative inside, exactly zero on the surface.
//! The normal is equally simple: the unit vector pointing from the origin to p.

use glam::Vec3;

use crate::sdf::Sdf;

/// SDF for a sphere centred at the origin.
///
/// `ph.Object(ph.SPHERE, radius)` maps to `SphereSdf::new(radius)`.
///
/// # Coordinate convention
///
/// The sphere is centred at the origin with uniform radius in all directions.
/// To move it, wrap in a [`crate::sdf::transform::Translate`].
pub struct SphereSdf {
    radius: f32,
}

impl SphereSdf {
    /// Create from a radius.
    #[inline]
    pub fn new(radius: f32) -> Self {
        Self { radius }
    }

    /// Return the radius.
    #[inline]
    pub fn radius(&self) -> f32 {
        self.radius
    }
}

impl Sdf for SphereSdf {
    /// Exact signed distance from `p` to the sphere surface.
    #[inline]
    fn distance(&self, p: Vec3) -> f32 {
        p.length() - self.radius
    }

    /// Exact analytical normal — the outward unit radial vector.
    #[inline]
    fn normal(&self, p: Vec3) -> Vec3 {
        p.normalize_or_zero()
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    fn sphere() -> SphereSdf { SphereSdf::new(5.0) }

    #[test]
    fn center_is_inside() {
        assert!(sphere().distance(Vec3::ZERO) < 0.0);
    }

    #[test]
    fn center_distance_equals_negative_radius() {
        assert_abs_diff_eq!(sphere().distance(Vec3::ZERO), -5.0, epsilon = 1e-5);
    }

    #[test]
    fn surface_is_zero_on_all_axes() {
        let s = sphere();
        for p in [
            Vec3::new( 5.0,  0.0,  0.0),
            Vec3::new(-5.0,  0.0,  0.0),
            Vec3::new( 0.0,  5.0,  0.0),
            Vec3::new( 0.0, -5.0,  0.0),
            Vec3::new( 0.0,  0.0,  5.0),
            Vec3::new( 0.0,  0.0, -5.0),
        ] {
            let d = s.distance(p);
            assert!(d.abs() < 1e-5, "{p:?}: expected ~0, got {d}");
        }
    }

    #[test]
    fn surface_is_zero_on_diagonal() {
        // Point on surface along unit diagonal: p = r * normalize(1,1,1).
        let r = 5.0_f32;
        let p = Vec3::splat(1.0).normalize() * r;
        assert_abs_diff_eq!(sphere().distance(p), 0.0, epsilon = 1e-5);
    }

    #[test]
    fn outside_is_positive() {
        // 3 mm past the surface along +X.
        assert_abs_diff_eq!(sphere().distance(Vec3::new(8.0, 0.0, 0.0)), 3.0, epsilon = 1e-5);
    }

    #[test]
    fn inside_near_surface_is_negative() {
        // 1 mm inside the surface along +X.
        assert_abs_diff_eq!(sphere().distance(Vec3::new(4.0, 0.0, 0.0)), -1.0, epsilon = 1e-5);
    }

    #[test]
    fn distance_is_isotropic() {
        // Sphere SDF must give the same value for all points equidistant from origin.
        let s = sphere();
        let r = 8.0_f32;
        let expected = r - 5.0;
        for p in [
            Vec3::new(r, 0.0, 0.0),
            Vec3::new(0.0, r, 0.0),
            Vec3::new(0.0, 0.0, r),
            Vec3::new(r, 0.0, 0.0) * -1.0,
        ] {
            assert_abs_diff_eq!(s.distance(p), expected, epsilon = 1e-5);
        }
    }

    #[test]
    fn radius_accessor() {
        assert_abs_diff_eq!(SphereSdf::new(12.5).radius(), 12.5, epsilon = 1e-5);
    }

    #[test]
    fn normal_points_outward() {
        let s = sphere();
        let n = s.normal(Vec3::new(5.0, 0.0, 0.0));
        assert_abs_diff_eq!(n.x, 1.0, epsilon = 1e-5);
        assert_abs_diff_eq!(n.y, 0.0, epsilon = 1e-5);
        assert_abs_diff_eq!(n.z, 0.0, epsilon = 1e-5);
    }

    #[test]
    fn normal_on_diagonal_is_unit_diagonal() {
        let s = sphere();
        let p = Vec3::splat(1.0).normalize() * 5.0;
        let n = s.normal(p);
        let expected = Vec3::splat(1.0).normalize();
        assert_abs_diff_eq!(n.x, expected.x, epsilon = 1e-5);
        assert_abs_diff_eq!(n.y, expected.y, epsilon = 1e-5);
        assert_abs_diff_eq!(n.z, expected.z, epsilon = 1e-5);
    }

    #[test]
    fn sphere_as_sdf_node() {
        use crate::sdf::SdfNode;
        let node: SdfNode = Box::new(SphereSdf::new(5.0));
        assert!(node.distance(Vec3::ZERO) < 0.0);
        assert!(node.distance(Vec3::new(10.0, 0.0, 0.0)) > 0.0);
    }
}
